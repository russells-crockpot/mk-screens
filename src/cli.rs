use crate::{settings::Settings, util::ENV, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use indicatif_log_bridge::LogWrapper;

lazy_static::lazy_static! {
    pub (crate)static ref PROGRESS_BAR_STYLE: ProgressStyle = ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.cyan/blue} {percent:3}% | {wide_msg}")
        .unwrap();
    pub (crate)static ref ERROR_PROGRESS_BAR_STYLE: ProgressStyle = ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.red/red} {percent:3}% | {wide_msg}")
        .unwrap()
        .progress_chars("███");
}

pub trait MultiProgressExt {
    fn new_default_progress_bar(&self, settings: &Settings) -> ProgressBar;
}

impl MultiProgressExt for MultiProgress {
    fn new_default_progress_bar(&self, settings: &Settings) -> ProgressBar {
        let pbar = self.add(ProgressBar::new((settings.num_captures() + 2) as u64));
        pbar.set_style(PROGRESS_BAR_STYLE.clone());
        pbar
    }
}

pub(crate) fn default_multi_progress() -> Result<MultiProgress> {
    let mp = if ENV.hide_progress_bars() {
        MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
    } else {
        let mp = MultiProgress::new();
        let logger = pretty_env_logger::formatted_builder().build();
        LogWrapper::new(mp.clone(), logger).try_init()?;
        mp
    };
    mp.set_move_cursor(false);
    Ok(mp)
}
