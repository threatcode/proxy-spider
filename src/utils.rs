pub trait CompactStrJoin {
    fn join_compact_str(self, sep: &str) -> compact_str::CompactString;
}

impl<I, T> CompactStrJoin for I
where
    I: Iterator<Item = T>,
    T: std::fmt::Display,
{
    fn join_compact_str(self, sep: &str) -> compact_str::CompactString {
        let mut s = compact_str::CompactString::default();
        let mut first = true;
        for item in self {
            if first {
                first = false;
            } else {
                s.push_str(sep);
            }
            // Using write! or similar mechanism if T doesn't have a direct to_string for CompactString
            // But Display trait works with fmt::Write for CompactString if implemented or manually pushed
            // CompactString implements fmt::Write
            use std::fmt::Write as _;
            write!(s, "{item}").expect("fmt::Write should not fail for CompactString");
        }
        s
    }
}

pub async fn is_docker() -> bool {
    #[cfg(target_os = "linux")]
    {
        static CACHE: tokio::sync::OnceCell<bool> =
            tokio::sync::OnceCell::const_new();

        *CACHE
            .get_or_init(async || {
                tokio::fs::try_exists("/.dockerenv").await.unwrap_or(false)
            })
            .await
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

pub fn pretty_error(e: &crate::Error) -> compact_str::CompactString {
    e.chain().join_compact_str(" \u{2192} ")
}
