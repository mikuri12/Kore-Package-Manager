pub mod appimage;
pub mod desktop;
pub mod extract;
pub mod nixos;
pub mod operations;
pub mod resolve;
pub mod update;
pub mod utils;

pub use appimage::*;
pub use desktop::*;
pub use extract::*;
pub use operations::*;
pub use resolve::*;
pub use update::*;
pub use utils::*;

pub enum InstallMessage {
    Progress(String, f64),
    SelectAsset(Vec<String>, tokio::sync::oneshot::Sender<usize>),
    SelectBinary(Vec<String>, tokio::sync::oneshot::Sender<usize>),
    SelectDesktop(Vec<String>, tokio::sync::oneshot::Sender<usize>),
}
