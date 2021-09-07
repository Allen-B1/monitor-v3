use std::borrow::Cow;
use std::convert::TryInto;
use std::error::Error;
use std::num::ParseIntError;
use std::process::Command;

pub fn get_active_window() -> Result<u32, Box<dyn Error>> {
    let output = Command::new("xprop").args(&["-root", "32x", "|$0", "_NET_ACTIVE_WINDOW"]).output()?;
    let res = output.stdout.split(|&c| c == '|' as u8).skip(1).next().ok_or("error parsing _NET_ACTIVE_WINDOW")?;
    Ok(u32::from_str_radix(std::str::from_utf8(&res[2..])?, 16)?)
}

pub fn get_all_windows() -> Result<Vec<u32>, Box<dyn Error>> {
    let output = Command::new("xprop").args(
        &["-root", "|$0+", "_NET_CLIENT_LIST"])
        .output()?;
    let res = std::str::from_utf8(output.stdout.split(|&c| c == '|' as u8)
        .skip(1)
        .next().ok_or("error parsing _NET_CLIENT_LIST")?)?.split(", ")
        .map(|x| u32::from_str_radix(&x[2..], 16));
    Ok(res.collect::<Result<Vec<u32>, ParseIntError>>()?)
}

#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub program: String,
    pub title: String,
}

pub fn get_window_info(wid: u32) -> Result<Option<WindowInfo>, Box<dyn Error>> {
    let output = Command::new("xprop").args(&[
        "-id", format!("0x{:x}", wid).as_str(),
        "-f", "_NET_WM_NAME", "8u", "|$0|",
        "-f", "WM_CLASS", "8s", "|$1|",
        "-f", "_NET_WM_WINDOW_TYPE", "32a", "|$0",
        "_NET_WM_NAME", "WM_CLASS", "_NET_WM_WINDOW_TYPE"])
        .output()?;

    let splits = output.stdout.split(|&c| c == '|' as u8);
    let mut splits = splits.skip(1);
    let title = {
        let title = splits.next().ok_or("error parsing _NET_WM_NAME")?;
        std::str::from_utf8(&title[1..title.len()-1])?
    };
    let mut splits = splits.skip(1);
    let program = {
        let program = splits.next().ok_or("error parsing WM_CLASS")?;
        std::str::from_utf8(&program[1..program.len()-1])?
    };
    let mut splits = splits.skip(1);
    {
        let type_ = splits.next().ok_or("error parsing _NET_WM_WINDOW_TYPE")?;
        let type_str = std::str::from_utf8(type_)?;
        if type_str != "_NET_WM_WINDOW_TYPE_NORMAL" {
            return Ok(None);
        }
    }

    Ok(Some(WindowInfo{program: program.to_owned(), title: title.to_owned()}))
}

extern crate monitor;
impl monitor::RawWindowData for WindowInfo {
    fn program(&self) -> Cow<'_, str> {
        (&self.program).into()
    }

    fn title(&self) -> Cow<'_, str> {
        (&self.title).into()
    }
}