use serde_derive::Deserialize;

/// Error returned when fetching fails.
///
/// This currently implements only `Debug` and `Display`. New traits/methods may be implemented in
/// the future.
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct Error (anyhow::Error);

/// Food within daily menu.
pub struct MenuItem {
    /// Description of the food
    pub description: String,
    /// Food price.
    ///
    /// Note that sometimes the price may be empty!
    pub price: String,
}

/// Daily menu of a restaurant.
///
/// This is a menu for specific day.
pub struct Menu {
    /// Date of the menu.
    pub date: String,
    /// Food items offered at this day.
    pub items: Vec<MenuItem>,
}

/// Fetches daily manu of given restaurant.
///
/// You can get restaurant name by visiting it at Zomato using browser and copying it from the URL.
///
/// This returns heap-allocated menu, because iterator would require self-referential return value.
/// If you happen to have some clever idea to work around this, I'll happily accept a PR.
pub async fn get_daily_menu(city: &str, restaurant: &str) -> Result<Vec<Menu>, Error> {
    get_daily_menu_internal(city, restaurant).await.map_err(Error)
}

#[derive(Deserialize, Debug)]
struct InternalMenuItem {
    name: String,
    #[serde(rename = "displayPrice")]
    price: String,
}

#[derive(Deserialize)]
struct DailyMenu {
    dishes: Vec<InternalMenuItem>,
    #[serde(rename = "timeHeading")]
    date: String,
}

#[derive(Deserialize)]
struct Sections {
    #[serde(rename = "SECTION_DAILY_MENU")]
    daily_menu: Vec<DailyMenu>,
}

#[derive(Deserialize)]
struct UnknownObject {
    sections: Sections,
}

#[derive(Deserialize)]
struct Pages {
    restaurant: std::collections::HashMap<String, UnknownObject>,
}

#[derive(Deserialize)]
struct Data {
    pages: Pages,
}

// We use internal function with `anyhow::Error` for convenience and it gets translated into our
// `Error` in the public function. This allows us to maintain ability to extend error type with
// information, while making it easy to write the initial version of library.
async fn get_daily_menu_internal(city: &str, restaurant: &str) -> Result<Vec<Menu>, anyhow::Error> {
    use scraper::Selector;
    use anyhow::Context;

    let url = format!("https://www.zomato.com/{}/{}/daily-menu", city, restaurant);
    #[cfg(feature = "debug-log")]
    let verbose = true;
    #[cfg(not(feature = "debug-log"))]
    let verbose = false;
    let req_builder = reqwest::Client::builder()
        .connection_verbose(verbose)
        .build()?
        .request(reqwest::Method::GET, &url)
        // I found that zomato server has some problems when some headers are passed,
        // so I copied everything from Mozilla Firefox.
        .header("User-Agent", "Mozilla/5.0 (X11; Fedora; Linux x86_64; rv:60.0) Gecko/20100101 Firefox/60.0")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        // This seems to be important
        .header("Accept-Encoding", "identity")
        // keep-alive must be lower case - not Keep-Alive!
        .header("Connection", "keep-alive")
        .header("DNT", "1")
        .header("Upgrade-Insecure-Requests", "1")
        .header("Cache-Control", "max-age=0")
        .header("Accept-Language", "en-US,en;q=0.5");

    let response = req_builder.send()
        .await?
        .bytes()
        .await?;
    let response_decoded = std::str::from_utf8(&response)?;
    let html = scraper::Html::parse_document(response_decoded);
    let script = html
        .select(&Selector::parse("script").unwrap())
        .into_iter()
        .filter_map(|script| script.text().next())
        .find(|script| script.contains("window.__PRELOADED_STATE__ = JSON.parse(\""))
        .ok_or_else(|| anyhow::anyhow!("data not found"))?;

    let mut iter = script.split("window.__PRELOADED_STATE__ = JSON.parse(\"");
    iter.next().expect("empty split");
    let json_with_tail = iter.next().expect("missing pattern");
    let json_escaped = json_with_tail.split("\")\n").next().expect("empty split");
    let mut json_unescaped = String::with_capacity(json_escaped.len());
    for piece in json_escaped.split("\\\"") {
        if !json_unescaped.is_empty() {
            json_unescaped.push_str("\"");
        }
        json_unescaped.push_str(piece);
    }
    let data = serde_json::from_str::<Data>(&json_unescaped).context("failed to parse json")?;
    let result = data
        .pages
        .restaurant
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing restaurant"))?
        .1
        .sections.daily_menu
        .into_iter()
        .map(|menu| {
            let items = menu
                .dishes
                .into_iter()
                .map(|item| MenuItem {
                    description: item.name,
                    price: item.price,
                })
                .collect::<Vec<_>>();
            Menu {
                items,
                date: menu.date,
            }
        })
        .collect::<Vec<_>>();

    Ok(result)
}
