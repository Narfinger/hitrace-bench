use std::{thread, time::Duration};

use crate::{args::WebDriverCmd, device, run_runconfig_filters, runconfig::RunConfig};
use anyhow::{Context, Result, anyhow};
use log::info;
use serde_json::Value;
use thirtyfour::{By, Capabilities, Key, WebDriver};

/// Run a webdriver script defined in runargs and return the result of texts.
/// Because we can only run one instance per phone, we have to do a single tokio runtime
/// and make this sync instead of async.
pub(crate) fn run_webdriver(run_config: &RunConfig) -> Result<Value> {
    if run_config.webdriver_script.is_empty() {
        return Ok(Value::Null);
    }
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
        .block_on(async {
            device::forward_port(7000)?;
            let caps = Capabilities::new();

            let driver = WebDriver::new("http://127.0.0.1:7000", caps).await?;

            thread::sleep(Duration::from_secs(5));

            for arg in &run_config.webdriver_script {
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
                        info!("Element {:?}", element);
                        info!("Element {:?}", element.text().await);
                        info!("Element {:?}", element.value().await);

                        element.send_keys(Key::Enter).await?;
                            element
                            .click()
                            .await
                            .with_context(|| format!("Trying to click element {}", element))?;
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
                        driver.quit().await?;
                        return res;
                    }
                }
            }
            driver.quit().await?;
            Err(anyhow!(
                "You did not end with a JS execution to get some value"
            ))
        })
}
