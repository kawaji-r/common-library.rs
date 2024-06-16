//! ## Sample Code
//! ```
//! use common_library::scraping_wrapper;
//! use std::collections::HashMap;
//! 
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Init
//!     let dom_defs = HashMap::from([
//!         ("search_text_area".to_string(), "[title=\"検索\"]".to_string()),
//!         ("search_button".to_string(), "center:nth-child(1) [value=\"Google 検索\"]".to_string()),
//!         ("first_result".to_string(), "h3".to_string()),
//!     ]);
//!     let scrape_options = scraping_wrapper::ScrapeOption {
//!         dom_defs: Some(dom_defs),
//!         headless: Some(false),
//!         window_size: Some((1920, 1080)),
//!     };
//!     let wrapper = scraping_wrapper::ScrapingWrapper::new(scrape_options)?;
//! 
//!     // Define a set of operations in advance
//!     let operations_1 = vec![
//!         scraping_wrapper::Operation {
//!             method: scraping_wrapper::OperationMethod::Go,
//!             target: "https://www.google.com/".to_string(),
//!             content: None,
//!         },
//!         scraping_wrapper::Operation {
//!             method: scraping_wrapper::OperationMethod::Fill,
//!             target: "search_text_area".to_string(),
//!             content: Some("sample text".to_string()),
//!         },
//!         scraping_wrapper::Operation {
//!             method: scraping_wrapper::OperationMethod::Click,
//!             target: "search_button".to_string(),
//!             content: None,
//!         },
//!     ];
//! 
//!     // Execute
//!     wrapper.operate(operations_1)?;
//! 
//!     // Get text
//!     let first_result = wrapper.get_inner_text("first_result")?;
//!     println!("first_result: {}", first_result);
//! 
//!     Ok(())
//! }
//! ```

use headless_chrome::{Browser, Element, LaunchOptionsBuilder, Tab};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

/// Structure to hold options for scraping
pub struct ScrapeOption {
    pub dom_defs: Option<HashMap<String, String>>, // Optional DOM definitions for scraping
    pub headless: Option<bool>, // Option to run browser in headless mode
    pub window_size: Option<(u32, u32)>,
}

/// Main structure to manage scraping operations
pub struct ScrapingWrapper {
    #[allow(dead_code)]
    browser: Browser, // Browser instance
    tab: Arc<Tab>, // Tab instance for operations
    dom_defs: HashMap<String, String>, // DOM definitions
}

/// Structure representing an operation to be performed
#[derive(Debug)]
pub struct Operation {
    pub method: OperationMethod, // Type of operation (Go, Click, Fill)
    pub target: String, // Target element or URL
    pub content: Option<String>, // Content to use in operation (e.g., text to fill)
}

/// Enum to define types of operations
#[derive(Debug)]
pub enum OperationMethod {
    Go, // Navigate to a URL
    Click, // Click on an element
    Fill, // Fill a text field
}

/// Function to retry a task multiple times with delays
pub fn retry<F, T>(mut task: F, retries: u8, delay: u64) -> Result<T, Box<dyn Error>>
where
    F: FnMut() -> Result<T, Box<dyn Error>>,
{
    let mut attempts = 0;
    while attempts < retries {
        match task() {
            Ok(result) => return Ok(result),
            Err(_) if attempts < retries - 1 => {
                sleep(Duration::from_secs(delay)); // Sleep before retrying
                attempts += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Err("Exceeded retry attempts".into()) // Return error if retries exceeded
}

/// Implementation of ScrapingWrapper
impl ScrapingWrapper {
    /// Constructor for ScrapingWrapper
    pub fn new(opt: ScrapeOption) -> Result<ScrapingWrapper, Box<dyn Error>> {
        let dom_defs = opt.dom_defs.unwrap_or(HashMap::new()); // Use provided DOM definitions or empty
        let headless = opt.headless.unwrap_or(true); // Default to headless mode if not specified
        let window_size = opt.window_size;

        // Build launch options for the browser
        let launch_options = LaunchOptionsBuilder::default()
            .headless(headless)
            .window_size(window_size) // Set window size
            .build()
            .expect("Failed to build browser launch options");
        let browser = Browser::new(launch_options)?; // Create new browser instance
        let tab = browser.new_tab()?; // Open a new tab

        Ok(ScrapingWrapper { browser, tab, dom_defs }) // Return new instance
    }

    /// Navigate to a URL
    pub fn go(&self, url: &str) -> Result<(), Box<dyn Error>> {
        let func = || {
            self.tab.navigate_to(url)?.wait_until_navigated()?; // Navigate and wait
            Ok(())
        };
        let result = retry(func, 5, 2)?; // Retry navigation if necessary
        Ok(result)
    }

    /// Display a dialog and wait for user interaction
    pub fn show_dialog_and_wait(&self, message: Option<&str>) -> Result<(), Box<dyn Error>> {
        let dialog_message = message.unwrap_or("Please press OK to continue."); // Default message
        self.tab.evaluate(&format!("alert('{}');", dialog_message), true)?; // Show alert dialog
        Ok(())
    }

    /// Retrieve a DOM element by target identifier
    pub fn get_dom(&self, target: &str) -> Result<Element, Box<dyn Error>> {
        let func = || {
            let selector = self.dom_defs.get(target).unwrap(); // Get selector from definitions
            let element = self.tab.wait_for_element_with_custom_timeout(selector, Duration::from_secs(1))?; // Wait for element
            element.scroll_into_view()?; // Scroll element into view
            Ok(element)
        };
        let result = retry(func, 5, 2)?; // Retry if necessary
        Ok(result)
    }

    /// Click on a specified element
    pub fn click(&self, element: Element) -> Result<(), Box<dyn Error>> {
        let func = || {
            element.click()?; // Perform click
            self.tab.wait_until_navigated()?; // Wait for navigation
            Ok(())
        };
        let result = retry(func, 5, 2)?; // Retry if necessary
        Ok(result)
    }

    /// Retrieve the inner text of a specified element
    pub fn get_inner_text(&self, target: &str) -> Result<String, Box<dyn Error>> {
        let func = || {
            let element = self.get_dom(target)?; // Get the DOM element
            let text = element.get_inner_text()?; // Get the inner text
            Ok(text)
        };
        let result = retry(func, 5, 2)?; // Retry if necessary
        Ok(result)
    }

    /// Fill a textbox with specified content
    pub fn fill_textbox(&self, element: Element, content: String) -> Result<(), Box<dyn Error>> {
        let func = || {
            element.type_into(&content)?; // Type content into the textbox
            Ok(())
        };
        let result = retry(func, 5, 2)?; // Retry if necessary
        Ok(result)
    }

    /// Retrieve a DOM element by text content, optional tag name, and index
    pub fn get_dom_by_text(&self, search_text: &str, tag_name: Option<&str>, index: Option<i8>) -> Result<Element, Box<dyn Error>> {
        let func = || {
            let _tag_name = tag_name.unwrap_or("*"); // Default to any tag
            let _index = index.unwrap_or(1); // Default to first element

            let _xpath = &format!("(//{}[normalize-space(text())='{}'])[{}]", _tag_name, search_text, _index); // Construct XPath
            let element = self.tab.wait_for_xpath_with_custom_timeout(_xpath, Duration::from_secs(1))?; // Wait for element
            element.scroll_into_view()?; // Scroll element into view
            Ok(element)
        };
        let result = retry(func, 5, 2)?; // Retry if necessary
        Ok(result)
    }

    /// Perform a series of operations
    pub fn operate(&self, operations: Vec<Operation>) -> Result<(), Box<dyn Error>> {
        for operation in operations {
            match operation.method {
                OperationMethod::Go => {
                    self.go(&operation.target)?; // Navigate to URL
                }
                OperationMethod::Click => {
                    let element = self.get_dom(&operation.target)?; // Get the element
                    self.click(element)?; // Click the element
                }
                OperationMethod::Fill => {
                    if let Some(content) = operation.content {
                        let element = self.get_dom(&operation.target)?; // Get the element
                        self.fill_textbox(element, content)?; // Fill the textbox
                    }
                }
            }
        }
        Ok(())
    }
}
