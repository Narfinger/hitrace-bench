use std::{thread, time::Duration};

use crate::args::{RunArgs, WebDriverCmd};
use anyhow::{Context, Result, anyhow};
use webdriver_client::{Driver, HttpDriverBuilder, LocationStrategy, messages::NewSessionCmd};

/// Run a webdriver script defined in runargs and return the result of texts.
pub(crate) fn run_webdriver(runargs: &RunArgs) -> Result<Vec<String>> {
    let Some(ref webdriver_cmds) = runargs.webdriver else {
        log::error!("No webdriver commands given");
        return Err(anyhow!("No webdriver commands were given"));
    };
    let driver = HttpDriverBuilder::default()
        .url("http://127.0.0.1:7000")
        .build()
        .map_err(|_| anyhow!("Could not connect Webdriver"))?;

    let mut params = NewSessionCmd::default();
    params.reset_always_match();

    thread::sleep(Duration::from_secs(5));

    let session = driver.session(&params)?;

    let mut text_vector = Vec::new();
    for arg in webdriver_cmds {
        match arg {
            WebDriverCmd::Click(element) => {
                let element = session
                    .find_element(&element, LocationStrategy::Css)
                    .context("Trying to find element {element}")?;
                element
                    .click()
                    .context("Trying to click element {element}")?;
            }
            WebDriverCmd::Sleep(seconds) => {
                thread::sleep(Duration::from_secs(*seconds));
            }
            WebDriverCmd::Text(element) => {
                let element = session
                    .find_element(&element, LocationStrategy::Css)
                    .context("Trying to find elmeent {element}")?;
                let text = element
                    .text()
                    .context("Trying to get text of element {element}")?;
                text_vector.push(text);
            }
        }
    }
    Ok(text_vector)
}
