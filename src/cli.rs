use indicatif::ProgressStyle;

lazy_static::lazy_static! {
    pub (crate)static ref PROGRESS_BAR_STYLE: ProgressStyle = ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.cyan/blue} {percent:3}% | {prefix}: {wide_msg}")
        .unwrap();
    pub (crate)static ref ERROR_PROGRESS_BAR_STYLE: ProgressStyle = ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.red/red} {percent:3}% | {wide_msg}")
        .unwrap()
        .progress_chars("███");
}
