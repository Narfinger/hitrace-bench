use std::{thread, time::Duration};

use crate::{args::WebDriverCmd, device, runconfig::RunConfig};
use anyhow::{Context, Result, anyhow};
use log::info;
use serde_json::Value;
use thirtyfour::{By, Capabilities, WebDriver};

/// Run a webdriver script defined in runargs and return the result of texts.
/// Because we can only run one instance per phone, we have to do a single tokio runtime
/// and make this sync instead of async.
pub(crate) fn run_webdriver(run_config: &RunConfig) -> Result<(String, Value)> {
    let webdriver_script = run_config.webdriver_script.as_ref().expect("Should have unwrapped earlier");
    info!(
        "Running webdriver"
    );
    device::just_start(&run_config.run_args, "https://www.servo.org".into())
        .context("Starting app failed")?;
    thread::sleep(Duration::from_secs(5));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_one_webdriver(webdriver_script))
}

async fn run_one_webdriver(webdriver_script: &crate::args::WebDriverScript) -> Result<(String, Value)> {
    device::forward_port(7000)?;
    let caps = Capabilities::new();
    let driver = WebDriver::new("http://127.0.0.1:7000", caps).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    for arg in &webdriver_script.cmds {
        info!("Running webdriver cmd {:?}", arg);
        match arg {
            WebDriverCmd::GoTo(url) => {
                driver.goto(url).await?;
            }
            WebDriverCmd::Click(element_class) => {
                let element = driver
                    .find(By::Css(element_class))
                    .await
                    .with_context(|| format!("Trying to find element {}", element_class))?;
                    element
                    .click()
                    .await
                    .with_context(|| format!("Trying to click element {}", element))?;
            }
            WebDriverCmd::Sleep(seconds) => {
                tokio::time::sleep(Duration::from_secs(*seconds)).await;
            }
            WebDriverCmd::ExecJS(s) => {
                info!("Executiong \"{}\"", s);
                let res = driver
                    .execute(s, Vec::new())
                    .await;
                info!("Returned value {:?}", res);
                driver.quit().await?;
                return res.map(|value| (webdriver_script.name.to_owned(), value.json().to_owned())).map_err(|e| e.into());
            }
        }
    }
    driver.quit().await?;
    Err(anyhow!(
        "You did not end with a JS execution to get some value"
    ))
}
