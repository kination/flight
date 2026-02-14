// Flight - XDP/AF_XDP based kernel-bypass networking library
//
// API structure (from XDP_MODULE_DESIGN.md):
//   Context  → 설정/정책 (Mode, Action, Rule)
//   Program  → 프로그램 로드/어태치/송수신
//   Stats    → 패킷 처리 통계
//
// 현재: std::net::UdpSocket 기반 fallback 백엔드 (macOS/Linux 모두 동작)
// 추후: Linux에서 aya 기반 XDP 백엔드로 교체

mod context;
pub mod debug;
mod error;
mod program;
mod stats;

pub use context::{Action, Context, Mode, Rule};
pub use error::FlightError;
pub use program::Program;
pub use stats::Stats;
