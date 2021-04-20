use std::{
  collections::hash_map::HashMap,
  os::raw::{c_int, c_void},
  sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Arc, Mutex,
  },
  thread,
};

use super::traits::*;

pub mod modifiers {
  pub const ALT: u32 = 2048;
  pub const ALT_GR: u32 = 16384;
  pub const CONTROL: u32 = 4096;
  pub const SHIFT: u32 = 512;
  pub const SUPER: u32 = 256;
}

pub mod keys {
  pub const BACKSPACE: u32 = 0x33;
  pub const TAB: u32 = 0x30;
  pub const ENTER: u32 = 0x24;
  pub const CAPS_LOCK: u32 = 0x39;
  pub const ESCAPE: u32 = 0x35;
  pub const SPACEBAR: u32 = 0x31;
  pub const PAGE_UP: u32 = 0x74;
  pub const PAGE_DOWN: u32 = 0x79;
  pub const END: u32 = 0x77;
  pub const HOME: u32 = 0x73;
  pub const ARROW_LEFT: u32 = 0x7B;
  pub const ARROW_RIGHT: u32 = 0x7C;
  pub const ARROW_UP: u32 = 0x7E;
  pub const ARROW_DOWN: u32 = 0x7D;
  pub const PRINT_SCREEN: u32 = 0xDEAD;
  pub const DELETE: u32 = 0x75;
  pub const SCROLL_LOCK: u32 = 0x6B; // F14
  pub const HELP: u32 = 0x72;
  // TODO
  // pub const NUMLOCK: u32 = 0;
  // Media
  pub const VOLUME_MUTE: u32 = 0x4A;
  pub const VOLUME_DOWN: u32 = 0x49;
  pub const VOLUME_UP: u32 = 0x48;
  // TODO
  /* pub const MEDIA_NEXT: u32 = 0;
  pub const MEDIA_PREV: u32 = 0;
  pub const MEDIA_STOP: u32 = 0;
  pub const MEDIA_PLAY_PAUSE: u32 = 0;
  pub const LAUNCH_MAIL: u32 = 0;*/
  // F1-F12
  pub const F1: u32 = 122;
  pub const F2: u32 = 120;
  pub const F3: u32 = 99;
  pub const F4: u32 = 118;
  pub const F5: u32 = 96;
  pub const F6: u32 = 97;
  pub const F7: u32 = 98;
  pub const F8: u32 = 100;
  pub const F9: u32 = 101;
  pub const F10: u32 = 109;
  pub const F11: u32 = 103;
  pub const F12: u32 = 111;
  // Numpad
  pub const DECIMAL: u32 = 0x41;
  pub const MULTIPLY: u32 = 0x43;
  pub const ADD: u32 = 0x45;
  pub const CLEAR: u32 = 0x47;
  pub const DIVIDE: u32 = 0x4B;
  pub const SUBTRACT: u32 = 0x4E;
  pub const KEYPAD_EQUALS: u32 = 0x51;
  pub const NUMPAD0: u32 = 0x52;
  pub const NUMPAD1: u32 = 0x53;
  pub const NUMPAD2: u32 = 0x54;
  pub const NUMPAD3: u32 = 0x55;
  pub const NUMPAD4: u32 = 0x56;
  pub const NUMPAD5: u32 = 0x57;
  pub const NUMPAD6: u32 = 0x58;
  pub const NUMPAD7: u32 = 0x59;
  pub const NUMPAD8: u32 = 0x5B;
  pub const NUMPAD9: u32 = 0x5C;
  pub const KEY_0: u32 = 0x1D;
  pub const KEY_1: u32 = 0x12;
  pub const KEY_2: u32 = 0x13;
  pub const KEY_3: u32 = 0x14;
  pub const KEY_4: u32 = 0x15;
  pub const KEY_5: u32 = 0x17;
  pub const KEY_6: u32 = 0x16;
  pub const KEY_7: u32 = 0x1A;
  pub const KEY_8: u32 = 0x1C;
  pub const KEY_9: u32 = 0x19;
  pub const A: u32 = 0x00;
  pub const B: u32 = 0x0B;
  pub const C: u32 = 0x08;
  pub const D: u32 = 0x02;
  pub const E: u32 = 0x0E;
  pub const F: u32 = 0x03;
  pub const G: u32 = 0x05;
  pub const H: u32 = 0x04;
  pub const I: u32 = 0x22;
  pub const J: u32 = 0x26;
  pub const K: u32 = 0x28;
  pub const L: u32 = 0x25;
  pub const M: u32 = 0x2E;
  pub const N: u32 = 0x2D;
  pub const O: u32 = 0x1F;
  pub const P: u32 = 0x23;
  pub const Q: u32 = 0x0C;
  pub const R: u32 = 0x0F;
  pub const S: u32 = 0x01;
  pub const T: u32 = 0x11;
  pub const U: u32 = 0x20;
  pub const V: u32 = 0x09;
  pub const W: u32 = 0x0D;
  pub const X: u32 = 0x07;
  pub const Y: u32 = 0x10;
  pub const Z: u32 = 0x06;
  pub const EQUAL: u32 = 0x18;
  pub const MINUS: u32 = 0x1B;
  pub const SINGLE_QUOTE: u32 = 0x27;
  pub const COMMA: u32 = 0x2B;
  pub const PERIOD: u32 = 0x2F;
  pub const SEMICOLON: u32 = 41;
  pub const SLASH: u32 = 44;
  pub const OPEN_QUOTE: u32 = 50;
  pub const OPEN_BRACKET: u32 = 33;
  pub const BACK_SLASH: u32 = 42;
  pub const CLOSE_BRACKET: u32 = 30;
}

type KeyCallback = unsafe extern "C" fn(c_int, *mut c_void);

#[link(name = "carbon_hotkey_binding.a", kind = "static")]
extern "C" {
  fn install_event_handler(cb: KeyCallback, data: *mut c_void) -> *mut c_void;
  fn uninstall_event_handler(handler_ref: *mut c_void) -> c_int;
  fn register_hotkey(id: i32, modifier: i32, key: i32) -> *mut c_void;
  fn unregister_hotkey(hotkey_ref: *mut c_void) -> c_int;
}

unsafe extern "C" fn trampoline<F>(result: c_int, user_data: *mut c_void)
where
  F: FnMut(c_int) + 'static,
{
  let user_data = &mut *(user_data as *mut F);
  user_data(result);
}

fn get_trampoline<F>() -> KeyCallback
where
  F: FnMut(c_int) + 'static,
{
  trampoline::<F>
}

fn register_event_handler_callback<F>(handler: *mut F) -> *mut c_void
where
  F: FnMut(i32) + 'static + Sync + Send,
{
  unsafe {
    let cb = get_trampoline::<F>();

    install_event_handler(cb, handler as *mut c_void)
  }
}

type ListenerId = i32;

#[derive(Debug)]
enum HotkeyMessage {
  RegisterHotkey(ListenerId, u32, u32),
  RegisterHotkeyResult(Result<(), HotkeyError>),
  UnregisterHotkey(ListenerId),
  UnregisterHotkeyResult(Result<(), HotkeyError>),
  DropThread,
}

struct CarbonRef(pub *mut c_void);
impl CarbonRef {
  pub fn new(start: *mut c_void) -> Self {
    CarbonRef(start)
  }
}
unsafe impl Sync for CarbonRef {}
unsafe impl Send for CarbonRef {}

type ListenerMap =
  Arc<Mutex<HashMap<ListenerId, (ListenerHotkey, Box<ListenerCallback>, CarbonRef)>>>;

pub struct Listener {
  last_id: ListenerId,
  handlers: ListenerMap,
  sender: Sender<HotkeyMessage>,
  receiver: Receiver<HotkeyMessage>,
}

impl HotkeyListener for Listener {
  fn new() -> Listener {
    let hotkeys = ListenerMap::default();

    let hotkey_map = hotkeys.clone();
    let (method_sender, thread_receiver) = mpsc::channel();
    let (thread_sender, method_receiver) = mpsc::channel();

    thread::spawn(move || {
      let hotkey_map_clone = hotkey_map.clone();
      let callback = Box::new(move |id| {
        if let Some((_, handler, _)) = hotkey_map_clone.lock().unwrap().get_mut(&id) {
          handler();
        }
      });

      let saved_callback = Box::into_raw(callback);
      let event_handler_ref = register_event_handler_callback(saved_callback);

      if event_handler_ref.is_null() {
        eprintln!("register_event_handler_callback failed!");
        let _ = unsafe { Box::from_raw(saved_callback) };
        return;
      }

      loop {
        match thread_receiver.recv() {
          Ok(HotkeyMessage::RegisterHotkey(id, modifiers, key)) => unsafe {
            let handler_ref = register_hotkey(id, modifiers as i32, key as i32);
            if handler_ref.is_null() {
              if let Err(err) = thread_sender.send(HotkeyMessage::RegisterHotkeyResult(Err(
                HotkeyError::BackendApiError(0),
              ))) {
                eprintln!("hotkey: thread_sender.send error {}", err);
              }
              continue;
            }
            if let Some((_, _, handler)) = hotkey_map.lock().unwrap().get_mut(&id) {
              *handler = CarbonRef::new(handler_ref);
            }
            if let Err(err) = thread_sender.send(HotkeyMessage::RegisterHotkeyResult(Ok(()))) {
              eprintln!("hotkey: thread_sender.send error {}", err);
            }
          },
          Ok(HotkeyMessage::UnregisterHotkey(id)) => unsafe {
            if let Some((_, _, handler_ref)) = hotkey_map.lock().unwrap().remove(&id) {
              let result = unregister_hotkey(handler_ref.0);
              if result != 0 {
                if let Err(err) = thread_sender.send(HotkeyMessage::UnregisterHotkeyResult(Err(
                  HotkeyError::BackendApiError(result as usize),
                ))) {
                  eprintln!("hotkey: thread_sender.send error {}", err);
                }
              } else if let Err(err) =
                thread_sender.send(HotkeyMessage::UnregisterHotkeyResult(Ok(())))
              {
                eprintln!("hotkey: thread_sender.send error {}", err);
              }
            } else {
              panic!("hotkey should be never be none");
            }
          },
          Ok(HotkeyMessage::DropThread) => unsafe {
            for (_, _, handler_ref) in hotkey_map.lock().unwrap().values() {
              let result = unregister_hotkey(handler_ref.0);
              if result != 0 {
                eprintln!("drop: unregister_hotkey failed: {}", result);
              }
            }
            let result = uninstall_event_handler(event_handler_ref);
            if result != 0 {
              eprintln!("drop: uninstall_event_handler failed: {}", result);
            }
            let _ = Box::from_raw(saved_callback);
            break;
          },
          Err(err) => {
            eprintln!("hotkey: try_recv error {}", err);
          }
          _ => unreachable!("other message should not arrive"),
        }
      }
    });

    Listener {
      sender: method_sender,
      receiver: method_receiver,
      handlers: hotkeys,
      last_id: 0,
    }
  }

  fn register_hotkey<F>(&mut self, hotkey: ListenerHotkey, handler: F) -> Result<(), HotkeyError>
  where
    F: 'static + FnMut() + Send,
  {
    for (key, _, _) in self.handlers.lock().unwrap().values() {
      if *key == hotkey {
        return Err(HotkeyError::HotkeyAlreadyRegistered(hotkey));
      }
    }
    self.last_id += 1;
    let id = self.last_id;
    self.handlers.lock().unwrap().insert(
      id,
      (
        hotkey,
        Box::new(handler),
        CarbonRef::new(std::ptr::null_mut()),
      ),
    );
    self
      .sender
      .send(HotkeyMessage::RegisterHotkey(
        id,
        hotkey.modifiers,
        hotkey.key,
      ))
      .map_err(|_| HotkeyError::ChannelError())?;

    let result = match self.receiver.recv() {
      Ok(HotkeyMessage::RegisterHotkeyResult(Ok(_))) => Ok(()),
      Ok(HotkeyMessage::RegisterHotkeyResult(Err(err))) => Err(err),
      Err(_) => Err(HotkeyError::ChannelError()),
      _ => Err(HotkeyError::Unknown),
    };
    if result.is_err() {
      self.handlers.lock().unwrap().remove(&id);
    }
    result
  }

  fn unregister_hotkey(&mut self, hotkey: ListenerHotkey) -> Result<(), HotkeyError> {
    let mut found_id = -1;
    for (id, (key, _, _)) in self.handlers.lock().unwrap().iter() {
      if *key == hotkey {
        found_id = *id;
        break;
      }
    }
    if found_id == -1 {
      return Err(HotkeyError::HotkeyNotRegistered(hotkey));
    }
    self
      .sender
      .send(HotkeyMessage::UnregisterHotkey(found_id))
      .map_err(|_| HotkeyError::ChannelError())?;
    match self.receiver.recv() {
      Ok(HotkeyMessage::UnregisterHotkeyResult(Ok(_))) => Ok(()),
      Ok(HotkeyMessage::UnregisterHotkeyResult(Err(err))) => Err(err),
      Err(_) => Err(HotkeyError::ChannelError()),
      _ => Err(HotkeyError::Unknown),
    }
  }

  fn registered_hotkeys(&self) -> Vec<ListenerHotkey> {
    let mut result = Vec::new();
    for v in self.handlers.lock().unwrap().values() {
      result.push(v.0);
    }
    result
  }
}

impl Drop for Listener {
  fn drop(&mut self) {
    if let Err(err) = self.sender.send(HotkeyMessage::DropThread) {
      eprintln!("cant send close thread message {}", err);
    }
  }
}
