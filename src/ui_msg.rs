//! 后台翻译任务 → UI 的消息。

use crate::model::TranslateOut;

#[derive(Debug)]
pub struct UiMsg {
    /// 发起本次翻译的请求序号，用于丢弃过期响应。
    pub seq: u64,
    pub result: Result<TranslateOut, String>,
}
