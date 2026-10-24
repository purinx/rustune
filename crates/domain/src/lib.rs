//! Rustuneのドメイン層。HTTP・保存・UI・OSには依存しない。
//! IDは呼び出し側が渡す。このcrateからOSの乱数を要求しない。

pub mod identity;
pub mod import_state;
pub mod metadata;
pub mod queue;
pub mod track;
