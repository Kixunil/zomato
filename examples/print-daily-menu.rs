#[tokio::main]
async fn main() -> Result<(), zomato::Error> {
    #[cfg(feature = "debug-log")]
    simple_logger::init().unwrap();

    let mut args = std::env::args();
    args.next();
    let city = args.next().expect("missing city and restaurant");
    let restaurant = args.next().expect("missing restaurant");

    let days = zomato::get_daily_menu(&city, &restaurant).await?;
    let width = days.iter().flat_map(|day| &day.items).map(|food| food.description.chars().count()).max();
    if let Some(width) = width {
        for menu in days {
            println!("{}", menu.date);
            for food in menu.items {
                print!("{}", food.description);
                for _ in 0..(width - food.description.chars().count()) {
                    print!(" ")
                }
                println!(" | {}", food.price);
            }
        }
    }
    Ok(())
}
