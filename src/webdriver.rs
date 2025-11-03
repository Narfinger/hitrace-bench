use std::{thread, time::Duration};

use crate::{args::WebDriverCmd, device, runconfig::RunConfig};
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use thirtyfour::{By, DesiredCapabilities, WebDriver};

/// Run a webdriver script defined in runargs and return the result of texts.
pub(crate) async fn run_webdriver(run_config: &RunConfig) -> Result<Value> {
    device::forward_port(7000)?;
    let caps = DesiredCapabilities::firefox();

    let driver = WebDriver::new("http://127.0.0.1:7000", caps).await?;

    thread::sleep(Duration::from_secs(5));

    for arg in &run_config.webdriver_script {
        match arg {
            WebDriverCmd::GoTo(url) => {
                driver.goto(url).await?;
            }
            WebDriverCmd::Click(element) => {
                let element = driver
                    .find(By::Id(element))
                    .await
                    .context("Trying to find element {element}")?;
                element
                    .click()
                    .await
                    .context("Trying to click element {element}")?;
            }
            WebDriverCmd::Sleep(seconds) => {
                thread::sleep(Duration::from_secs(*seconds));
            }
            WebDriverCmd::ExecJS(s) => {
                let res = driver
                    .execute(s, Vec::new())
                    .await
                    .map(|v| v.json().clone())
                    .context("Could not execute javascript");
                driver.quit();
                return res;
            }
        }
    }
    driver.quit();
    Err(anyhow!(
        "You did not end with a JS execution to get some value"
    ))
}
