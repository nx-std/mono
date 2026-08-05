//! The path buffer every command that names a file or directory carries.

/// Size of the path buffer `fsp-srv` commands take, terminator included.
///
/// Paths travel as a fixed-size buffer rather than a length-prefixed string, so
/// this is the buffer's size and not a maximum a shorter path may fall under.
pub const FS_MAX_PATH: usize = 0x301;
