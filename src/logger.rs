#[cfg(feature = "logger")]
pub fn set_logger() {
    use anstyle::{Color, Style};
    use chrono::Local;
    use log::{warn, Level};
    use std::io::Write;

    let init = env_logger::Builder::from_default_env()
        .format(|buf, record| {
            /* let green = Style::new().fg_color(Some(Color::Rgb((85, 170, 127).into()))); */
            let yellow = Style::new().fg_color(Some(Color::Rgb((255, 170, 127).into())));
            let red = Style::new().fg_color(Some(Color::Rgb((200, 85, 85).into())));

            let timestamp = Local::now().format("%m-%d %H:%M:%S");

            match record.level() {
                Level::Info => {
                    writeln!(buf, "[{timestamp}] {}", record.args())
                }
                Level::Debug => {
                    writeln!(
                        buf,
                        "{yellow}[Debug]{yellow:#} [{timestamp}]: {}",
                        record.args()
                    )
                }
                Level::Warn => {
                    writeln!(
                        buf,
                        "{yellow}[Warn]{yellow:#} [{timestamp}]: {}",
                        record.args()
                    )
                }
                Level::Error => {
                    writeln!(
                        buf,
                        "{red}[Error]{yellow:#} [{timestamp}]: {}",
                        record.args()
                    )
                }
                Level::Trace => {
                    writeln!(buf, "{red}[Trace]{red:#} [{timestamp}]: {}", record.args())
                }
            }
        })
        .try_init();

    if let Err(e) = init {
        warn!(
            "Kovi init env_logger failed: {}. Very likely you've already started a logger",
            e
        );
    }
}
