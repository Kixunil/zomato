use std::io;
use std::fmt;

trait TtsEngine {
    fn speak(&self, f: &mut dyn FnMut(&mut dyn fmt::Write) -> fmt::Result) -> io::Result<()>;
}

struct Festival {
    language: Option<String>,
}

impl TtsEngine for Festival {
    fn speak(&self, f: &mut dyn FnMut(&mut dyn fmt::Write) -> fmt::Result) -> io::Result<()> {
        use io::Write;

        let mut cmd = std::process::Command::new("festival");
        if let Some(language) = &self.language {
            cmd.arg("--language");
            cmd.arg(language);
        }
        cmd.arg("--tts");
        cmd.stdin(std::process::Stdio::piped());
        let mut child = cmd.spawn()?;
        let mut stdin = io::BufWriter::new(child.stdin.take().expect("bug in std"));
        fmt2io::write(&mut stdin, |mut w| f(&mut w))?;
        stdin.flush()?;
        // close
        std::mem::drop(stdin);
        child.wait()?;
        Ok(())
    }
}

struct Espeak {
    language: Option<String>,
    speed: Option<String>,
}

impl TtsEngine for Espeak {
    fn speak(&self, f: &mut dyn FnMut(&mut dyn fmt::Write) -> fmt::Result) -> io::Result<()> {
        use io::Write;

        let mut cmd = std::process::Command::new("espeak");
        if let Some(language) = &self.language {
            cmd.arg("-v");
            cmd.arg(language);
        }
        if let Some(speed) = &self.speed {
            cmd.arg("-s");
            cmd.arg(speed);
        }
        cmd.arg("--stdin");
        cmd.stdin(std::process::Stdio::piped());
        let mut child = cmd.spawn()?;
        let mut stdin = io::BufWriter::new(child.stdin.take().expect("bug in std"));
        fmt2io::write(&mut stdin, |mut w| f(&mut w))?;
        stdin.flush()?;
        // close
        std::mem::drop(stdin);
        child.wait()?;
        Ok(())
    }
}

struct Pico2Wave {
    language: Option<String>,
}

impl TtsEngine for Pico2Wave {
    fn speak(&self, f: &mut dyn FnMut(&mut dyn fmt::Write) -> fmt::Result) -> io::Result<()> {
        use mktemp::Temp;

        let mut text = String::new();
        f(&mut text).expect("writing to String never fails");

        let tmp_dir = Temp::new_dir()?;
        let tmp_file = tmp_dir.join("message.wav");

        let mut cmd = std::process::Command::new("pico2wave");
        if let Some(language) = &self.language {
            cmd.arg("-l");
            cmd.arg(language);
        }
        cmd.arg("-w");
        cmd.arg(&tmp_file);
        cmd.arg(&text);
        eprintln!("{:?}", cmd);
        let mut child = cmd.spawn()?;
        child.wait()?;
        let mut cmd = std::process::Command::new("aplay");
        cmd.arg(&tmp_file);
        let mut child = cmd.spawn()?;
        child.wait()?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), zomato::Error> {
    #[cfg(feature = "debug-log")]
    simple_logger::init().unwrap();

    let mut args = std::env::args().fuse();
    args.next();
    let city = args.next().expect("missing city, restaurant, and text-to-speech engine");
    let restaurant = args.next().expect("missing restaurant and text-to-speech engine");
    let tts_engine = args.next().expect("missing text-to-speech engine");

    let engine: Box<dyn TtsEngine> = match &*tts_engine {
        "festival" => Box::new(Festival { language: args.next() }),
        "espeak" => {
            let language = args.next();
            let speed = args.next();
            Box::new(Espeak { language, speed })
        },
        "pico2wave" => Box::new(Pico2Wave { language: args.next() }),
        x => {
            eprintln!("Unknown text-to-speech engine: '{}'", x);
            std::process::exit(1);
        },
    };

    if let Some(today) = zomato::get_daily_menu(&city, &restaurant).await?.into_iter().next() {
        engine.speak(&mut |writer| {
            for food in &today.items {
                write!(writer, "{} {} ", food.description, food.price)?;
            }
            Ok(())
        })
        .expect("Failed to speak");
    }
    Ok(())
}
