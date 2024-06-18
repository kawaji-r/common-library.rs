//! ## 現在用意しているメソッド
//! * ページ遷移
//! * ダイアログを表示してユーザー制御待ち
//! * DOM取得
//! * クリック
//! * innerText取得
//! * フォームにテキスト入力
//! * innerTextからDOM取得
//! * タブを閉じる
//! * 一連の操作をまとめて実行
//!
//! ## サンプルコード
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
//!         port_number: None,
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

/// スクレイピングのオプションを保持する構造体
pub struct ScrapeOption {
    pub dom_defs: Option<HashMap<String, String>>, // スクレイピングのためのオプションのDOM定義
    pub headless: Option<bool>,                    // ブラウザをヘッドレスモードで実行するオプション
    pub window_size: Option<(u32, u32)>,
    pub port_number: Option<u16>, // 実行中のブラウザを使用する場合、ポート番号を定義
                                  // `--remote-debugging-port`でブラウザを実行することで可能
}

/// スクレイピング操作を管理するためのメイン構造体
pub struct ScrapingWrapper {
    #[allow(dead_code)]
    browser: Browser, // ブラウザインスタンス
    tab: Arc<Tab>,                     // 操作用のタブインスタンス
    dom_defs: HashMap<String, String>, // DOM定義
}

/// 実行する操作を表す構造体
#[derive(Debug)]
pub struct Operation {
    pub method: OperationMethod, // 操作の種類 (Go, Click, Fill)
    pub target: String,          // 対象の要素またはURL
    pub content: Option<String>, // 操作に使用するコンテンツ (例: 入力するテキスト)
}

/// 操作の種類を定義する列挙型
#[derive(Debug)]
pub enum OperationMethod {
    Go,    // URLに移動
    Click, // 要素をクリック
    Fill,  // テキストフィールドに入力
}

/// 遅延を伴うタスクを複数回リトライする関数
pub fn retry<F, T>(mut task: F, retries: u8, delay: u64) -> Result<T, Box<dyn Error>>
where
    F: FnMut() -> Result<T, Box<dyn Error>>,
{
    let mut attempts = 0;
    while attempts < retries {
        match task() {
            Ok(result) => return Ok(result),
            Err(_) if attempts < retries - 1 => {
                sleep(Duration::from_secs(delay)); // リトライ前にスリープ
                attempts += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Err("リトライ回数を超えました".into()) // リトライ回数を超えた場合にエラーを返す
}

/// ScrapingWrapperの実装
impl ScrapingWrapper {
    /// ScrapingWrapperのコンストラクタ
    pub fn new(opt: ScrapeOption) -> Result<ScrapingWrapper, Box<dyn Error>> {
        let dom_defs = opt.dom_defs.unwrap_or(HashMap::new()); // 提供されたDOM定義または空の定義を使用

        let browser = match opt.port_number {
            Some(port_number) => {
                let browser_info_url =
                    format!("http://localhost:{}/json", port_number.to_string(),);
                let response = reqwest::blocking::get(&browser_info_url)?;
                let browser_info: serde_json::Value = serde_json::from_str(&response.text()?)?;
                let websocket_url = browser_info
                    .as_array()
                    .ok_or("ブラウザ情報が配列ではありません")?
                    .iter()
                    .find(|&info| info["type"] == "page")
                    .and_then(|info| info["webSocketDebuggerUrl"].as_str())
                    .ok_or("タイプ 'page' のWebSocket URLが見つかりません")?
                    .to_string();
                Browser::connect(websocket_url)?
            }
            None => {
                let headless = opt.headless.unwrap_or(true); // 指定がない場合はデフォルトでヘッドレスモード
                let window_size = opt.window_size;

                // ブラウザの起動オプションを構築
                let launch_options = LaunchOptionsBuilder::default()
                    .headless(headless)
                    .window_size(window_size) // ウィンドウサイズを設定
                    .build()
                    .expect("ブラウザ起動オプションの構築に失敗しました");
                Browser::new(launch_options)? // 新しいブラウザインスタンスを作成
            }
        };
        let tabs = browser.get_tabs();
        let tab = tabs.lock().unwrap().last().unwrap().clone();

        Ok(ScrapingWrapper {
            browser,
            tab: tab,
            dom_defs,
        }) // 新しいインスタンスを返す
    }

    /// URLに移動
    pub fn go(&self, url: &str) -> Result<(), Box<dyn Error>> {
        let func = || {
            // let tab = self.browser.new_tab()?; // 新しいタブを開く // TODO: 削除
            // self.tab = Some(tab); // TODO: 削除
            self.tab.navigate_to(url)?.wait_until_navigated()?; // 移動して待機
            Ok(())
        };
        let result = retry(func, 5, 2)?; // 必要に応じてリトライ
        Ok(result)
    }

    /// ダイアログを表示してユーザーの操作を待機
    pub fn show_dialog_and_wait(&self, message: Option<&str>) -> Result<(), Box<dyn Error>> {
        let dialog_message = message.unwrap_or("続行するにはOKを押してください。"); // デフォルトメッセージ
        self.tab
            .evaluate(&format!("alert('{}');", dialog_message), true)?; // アラートダイアログを表示
        Ok(())
    }

    /// ターゲット識別子でDOM要素を取得
    pub fn get_dom(&self, target: &str) -> Result<Element, Box<dyn Error>> {
        let func = || {
            let selector = self.dom_defs.get(target).unwrap(); // 定義からセレクタを取得
            let element = self
                .tab
                .wait_for_element_with_custom_timeout(selector, Duration::from_secs(1))?; // 要素を待機
            element.scroll_into_view()?; // 要素を表示領域にスクロール
            Ok(element)
        };
        let result = retry(func, 5, 2)?; // 必要に応じてリトライ
        Ok(result)
    }

    /// 指定された要素をクリック
    pub fn click(&self, element: Element) -> Result<(), Box<dyn Error>> {
        let func = || {
            element.click()?; // クリックを実行
            sleep(Duration::from_secs(1));
            self.tab.wait_until_navigated()?; // ナビゲーションを待機
            Ok(())
        };
        let result = retry(func, 5, 2)?; // 必要に応じてリトライ
        Ok(result)
    }

    /// 指定された要素の内部テキストを取得
    pub fn get_inner_text(&self, target: &str) -> Result<String, Box<dyn Error>> {
        let func = || {
            let element = self.get_dom(target)?; // DOM要素を取得
            let text = element.get_inner_text()?; // 内部テキストを取得
            Ok(text)
        };
        let result = retry(func, 5, 2)?; // 必要に応じてリトライ
        Ok(result)
    }

    /// 指定されたコンテンツでテキストボックスを埋める
    pub fn fill_textbox(&self, element: Element, content: String) -> Result<(), Box<dyn Error>> {
        let func = || {
            element.type_into(&content)?; // テキストボックスにコンテンツを入力
            Ok(())
        };
        let result = retry(func, 5, 2)?; // 必要に応じてリトライ
        Ok(result)
    }

    /// テキストコンテンツ、オプションのタグ名、およびインデックスでDOM要素を取得
    pub fn get_dom_by_text(
        &self,
        search_text: &str,
        tag_name: Option<&str>,
        index: Option<i8>,
    ) -> Result<Element, Box<dyn Error>> {
        let func = || {
            let _tag_name = tag_name.unwrap_or("*"); // デフォルトは任意のタグ
            let _index = index.unwrap_or(1); // デフォルトは最初の要素

            let _xpath = &format!(
                "(//{}[normalize-space(text())='{}'])[{}]",
                _tag_name, search_text, _index
            ); // XPathを構築
            let element = self
                .tab
                .wait_for_xpath_with_custom_timeout(_xpath, Duration::from_secs(1))?; // 要素を待機
            element.scroll_into_view()?; // 要素を表示領域にスクロール
            Ok(element)
        };
        let result = retry(func, 5, 2)?; // 必要に応じてリトライ
        Ok(result)
    }

    pub fn close_tab(&self) -> Result<(), Box<dyn Error>> {
        self.tab.close(true)?;
        Ok(())
    }

    /// 一連の操作を実行
    pub fn operate(&self, operations: Vec<Operation>) -> Result<(), Box<dyn Error>> {
        for operation in operations {
            match operation.method {
                OperationMethod::Go => {
                    self.go(&operation.target)?; // URLに移動
                }
                OperationMethod::Click => {
                    let element = self.get_dom(&operation.target)?; // 要素を取得
                    self.click(element)?; // 要素をクリック
                }
                OperationMethod::Fill => {
                    if let Some(content) = operation.content {
                        let element = self.get_dom(&operation.target)?; // 要素を取得
                        self.fill_textbox(element, content)?; // テキストボックスを埋める
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_exist_browser() {
        // 初期化
        let scrape_options = ScrapeOption {
            dom_defs: None,
            headless: Some(false),
            window_size: Some((1920, 1080)),
            port_number: Some(9222),
        };
        let wrapper = ScrapingWrapper::new(scrape_options).unwrap();
        wrapper.go("https://google.com").unwrap();
        wrapper.close_tab().unwrap();
        // ユーザーの目視確認とする
    }

    #[test]
    fn example_usage() {
        // 初期化
        let dom_defs = HashMap::from([
            (
                "search_text_area".to_string(),
                "[title=\"検索\"]".to_string(),
            ),
            (
                "search_button".to_string(),
                "center:nth-child(1) [value=\"Google 検索\"]".to_string(),
            ),
            ("first_result".to_string(), "h3".to_string()),
        ]);
        let scrape_options = ScrapeOption {
            dom_defs: Some(dom_defs),
            headless: Some(false),
            window_size: Some((1920, 1080)),
            port_number: None,
        };
        let wrapper = ScrapingWrapper::new(scrape_options).unwrap();

        // 一連の操作を事前に定義
        let operations_1 = vec![
            Operation {
                method: OperationMethod::Go,
                target: "https://www.google.com/".to_string(),
                content: None,
            },
            Operation {
                method: OperationMethod::Fill,
                target: "search_text_area".to_string(),
                content: Some("sample text".to_string()),
            },
            Operation {
                method: OperationMethod::Click,
                target: "search_button".to_string(),
                content: None,
            },
        ];

        // 実行
        wrapper.operate(operations_1).unwrap();

        // テキストを取得
        let first_result = wrapper.get_inner_text("first_result").unwrap();
        println!("first_result: {}", first_result);
    }
}
