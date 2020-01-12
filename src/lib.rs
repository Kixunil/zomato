use std::fmt;

/// Error returned when fetching fails.
///
/// This currently implements only `Debug` and `Display`. New traits/methods may be implemented in
/// the future.
pub struct Error (anyhow::Error);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

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

// We use internal function with `anyhow::Error` for convenience and it gets translated into our
// `Error` in the public function. This allows us to maintain ability to extend error type with
// information, while making it easy to write the initial version of library.
async fn get_daily_menu_internal(city: &str, restaurant: &str) -> Result<Vec<Menu>, anyhow::Error> {
    use scraper::Selector;

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
    let tmi_group_name = Selector::parse(".tmi-group-name").unwrap();
    let tmi_daily = Selector::parse(".tmi-daily").unwrap();
    let tmi_name = Selector::parse(".tmi-name").unwrap();
    let tmi_price = Selector::parse(".tmi-price div.row").unwrap();
    let result = html
        .select(&Selector::parse("#daily-menu-container .tmi-group").unwrap())
        .into_iter()
        .map(|day| {
            let date = day
                .select(&tmi_group_name)
                .into_iter()
                .next()
                .ok_or("missing group name")
                .map_err(anyhow::Error::msg)?
                .text()
                .next()
                .ok_or("missing text of group name")
                .map_err(anyhow::Error::msg)?
                .trim()
                .to_owned();

            let items = day
                .select(&tmi_daily)
                .into_iter()
                .map(|food| {
                    let food_description = food
                        .select(&tmi_name)
                        .into_iter()
                        .next()
                        .ok_or("missing food description")
                        .map_err(anyhow::Error::msg)?
                        .text()
                        .next()
                        .ok_or("missing text of food description")
                        .map_err(anyhow::Error::msg)?
                        .trim()
                        .to_owned();

                    let price = food
                        .select(&tmi_price)
                        .into_iter()
                        .next()
                        .ok_or("missing price")
                        .map_err(anyhow::Error::msg)?
                        .text()
                        .next()
                        .ok_or("missing text of price")
                        .map_err(anyhow::Error::msg)?
                        .trim()
                        .to_owned();

                    Ok(MenuItem { description: food_description, price })
                })
                .collect::<Result<_, anyhow::Error>>()?;
            Ok(Menu { date, items, })
        })
        .collect::<Result<_, anyhow::Error>>()?;
    Ok(result)
}
