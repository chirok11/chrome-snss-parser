use std::{
    env::args,
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt};

#[macro_use]
extern crate log;

#[derive(PartialEq, Debug)]
enum TabRestoreCommand {
    CommandSelectedNavigationInTab(/* index */ i32),
    CommandUpdateTabNavigation(
        /* tab_id */ i32,
        /* index */ i32,
        /* url */ String,
    ),
    CommandWindow,
    CommandAddTabExtraData,
    Marker,
    End,
}

fn check_headers<'a>(c: &'a mut Cursor<Vec<u8>>) -> Result<(), std::io::Error> {
    let mut signature = [0u8; 4];
    c.read(&mut signature)?;
    let version = c.read_i32::<LittleEndian>()?;
    debug!("signature={:?}", signature);
    debug!("version={}", version);
    assert!(&signature == &[83, 78, 83, 83]);

    Ok(())
}

fn read_string<'a>(c: &'a mut Cursor<Vec<u8>>) -> Result<String, std::io::Error> {
    let len = c.read_i32::<LittleEndian>()?;
    let mut buf = vec![0u8; len as usize];
    c.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf).expect("failed to parse string"))
}

fn read_command<'a>(c: &'a mut Cursor<Vec<u8>>) -> Result<TabRestoreCommand, std::io::Error> {
    let command_size = match c.read_u16::<LittleEndian>() {
        Ok(size) => size,
        Err(_) => return Ok(TabRestoreCommand::End),
    };
    let mut buf = vec![0u8; command_size as usize];
    c.read_exact(&mut buf)?;
    // debug!("command_size={}", command_size);
    // debug!("buffer={:?}", buf);
    let mut cbuf = Cursor::new(buf);

    let command_id = cbuf.read_u8()?;
    debug!("command_id={}", command_id);

    match command_id {
        1 => {
            let window_id = cbuf.read_i32::<LittleEndian>()?;
            debug!("UpdateWindow(");
            debug!("\twindow_id={}", window_id);
            let tab_id = cbuf.read_i32::<LittleEndian>()?;
            debug!("\ttab_id={}", tab_id);
            let index_ = cbuf.read_i32::<LittleEndian>()?;
            debug!("\tindex_={}", index_);
            let virtual_url = read_string(&mut cbuf)?;
            debug!("\tvirtual_url_={}", virtual_url);
            debug!(")");

            Ok(TabRestoreCommand::CommandUpdateTabNavigation(
                tab_id,
                index_,
                virtual_url,
            ))
        }
        4 => {
            let tab_id = cbuf.read_i32::<LittleEndian>()?;
            let index = cbuf.read_i32::<LittleEndian>()?;
            let timestamp = cbuf.read_i64::<LittleEndian>()?;
            debug!(
                "SelectedNavigationInTab(tab_id={}; index={}; timestamp={})",
                tab_id, index, timestamp
            );

            Ok(TabRestoreCommand::CommandSelectedNavigationInTab(index))
        }
        9 => {
            let window_id = cbuf.read_i64::<LittleEndian>()?;
            let selected_tab_index = cbuf.read_i32::<LittleEndian>()?;
            let num_tabs = cbuf.read_i32::<LittleEndian>()?;
            let timestamp = cbuf.read_i64::<LittleEndian>()?;
            let bounds_x = cbuf.read_i32::<LittleEndian>()?;
            let bounds_y = cbuf.read_i32::<LittleEndian>()?;
            let bounds_width = cbuf.read_i32::<LittleEndian>()?;
            let bounds_height = cbuf.read_i32::<LittleEndian>()?;
            let window_show_state = cbuf.read_i32::<LittleEndian>()?;
            // read workspace...
            let workspace = read_string(&mut cbuf)?;

            let ttype = cbuf.read_i32::<LittleEndian>()?;

            debug!(
                "CommandWindow(window_id={}; selected_tab_index={}; num_tabs={}; timestamp={}; bounds_pos={}x{}; bounds={}x{}; show_state={}; workspace={}; type={})",
                window_id, selected_tab_index, num_tabs, timestamp, bounds_x, bounds_y, bounds_width, bounds_height, window_show_state, &workspace, ttype
            );

            Ok(TabRestoreCommand::CommandWindow)
        }
        14 => Ok(TabRestoreCommand::CommandAddTabExtraData),
        255 => Ok(TabRestoreCommand::Marker),
        _ => {
            panic!("{} unimplemented", command_id);
        }
    }
}

fn parse_file<'a>(file: &'a str) -> Result<Vec<TabRestoreCommand>, std::io::Error> {
    debug!("reading file: {}", file);
    let mut fs = File::open(Path::new(file))?;
    let mut buf = vec![];

    let n = fs.read_to_end(&mut buf)?;
    debug!("readen {} bytes from file", n);
    let mut c = Cursor::new(buf);
    // reading two i32
    check_headers(&mut c)?;

    let mut commands = vec![];

    loop {
        let cmd = read_command(&mut c)?;
        if cmd == TabRestoreCommand::End {
            break;
        }
        commands.push(cmd);
    }

    Ok(commands)
}

fn main() {
    pretty_env_logger::init();
    let mut args = args();
    if args.len() != 2 {
        println!("Usage: ./bin [path]");
        return;
    }
    let path = args.nth(1).expect("use bin [path]");

    let cmds = parse_file(&path).expect("Failed to parse file");

    for command in cmds {
        println!("{:#?}", command);
    }
}
