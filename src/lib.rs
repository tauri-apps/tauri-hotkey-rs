use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use std::{
  collections::{hash_map::Entry, HashMap},
  fmt,
  hash::Hash,
  str::FromStr,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
};

use tauri_hotkey_sys::*;

type GlobalListener = Lazy<Arc<Mutex<Listener>>>;
type GlobalHotkeyMap =
  Arc<Mutex<HashMap<Hotkey, HashMap<usize, Box<dyn 'static + FnMut() + Send>>>>>;

static GLOBAL_LISTENER: GlobalListener = Lazy::new(|| Arc::new(Mutex::new(Listener::new())));
static GLOBAL_HOTKEY_MAP: Lazy<GlobalHotkeyMap> = Lazy::new(GlobalHotkeyMap::default);
static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct HotkeyManager {
  registered_hotkeys: Vec<Hotkey>,
  id: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("Hotkey system error: {0}")]
  System(#[from] HotkeyError),
  #[error("Hotkey already registered")]
  HotkeyAlreadyRegistered(Hotkey),
  #[error("Hotkey is not registered")]
  HotkeyNotRegistered(Hotkey),
  #[error("failed to parse hotkey: {0}")]
  InvalidHotkey(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Default for HotkeyManager {
  fn default() -> Self {
    Self {
      registered_hotkeys: Vec::new(),
      id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
    }
  }
}

impl HotkeyManager {
  pub fn new() -> Self {
    Default::default()
  }

  pub fn register<F>(&mut self, hotkey: Hotkey, callback: F) -> Result<()>
  where
    F: 'static + FnMut() + Send,
  {
    if self.registered_hotkeys.contains(&hotkey) {
      return Err(Error::HotkeyAlreadyRegistered(hotkey));
    }

    let hotkey_ = hotkey.clone();
    match GLOBAL_HOTKEY_MAP.lock().unwrap().entry(hotkey.clone()) {
      Entry::Occupied(mut entry) => {
        let entry = entry.get_mut();
        entry.insert(self.id, Box::new(callback));
      }
      Entry::Vacant(entry) => {
        GLOBAL_LISTENER.lock().unwrap().register_hotkey(
          ListenerHotkey::new(hotkey.modifier_as_flag(), hotkey.key as u32),
          move || {
            if let Some(entry) = GLOBAL_HOTKEY_MAP.lock().unwrap().get_mut(&hotkey) {
              for (_, cb) in entry.iter_mut() {
                cb();
              }
            }
          },
        )?;
        let mut new_map: HashMap<usize, Box<dyn 'static + FnMut() + Send>> = HashMap::new();
        new_map.insert(self.id, Box::new(callback));
        entry.insert(new_map);
      }
    }

    info!("register hotkey {}", hotkey_);
    self.registered_hotkeys.push(hotkey_);

    Ok(())
  }

  pub fn unregister(&mut self, hotkey: &Hotkey) -> Result<()> {
    match self.registered_hotkeys.iter().position(|h| h == hotkey) {
      Some(index) => {
        self.registered_hotkeys.remove(index);
      }
      None => return Err(Error::HotkeyNotRegistered(hotkey.clone())),
    }

    match GLOBAL_HOTKEY_MAP.lock().unwrap().entry(hotkey.clone()) {
      std::collections::hash_map::Entry::Occupied(mut occ_entry) => {
        let entry = occ_entry.get_mut();
        if entry.remove(&self.id).is_none() {
          panic!("should never be vacant");
        }
        if entry.is_empty() {
          occ_entry.remove_entry();
          GLOBAL_LISTENER
            .lock()
            .unwrap()
            .unregister_hotkey(ListenerHotkey::new(
              hotkey.modifier_as_flag(),
              hotkey.key as u32,
            ))?;
        }
      }
      std::collections::hash_map::Entry::Vacant(_) => {
        panic!("should never be vacant");
      }
    }
    info!("unregister hotkey {}", hotkey);
    Ok(())
  }

  pub fn unregister_all(&mut self) -> Result<()> {
    let mut result = Ok(());
    for hotkey in self.registered_hotkeys.clone().iter() {
      result = self.unregister(hotkey);
    }
    result
  }
}

impl Drop for HotkeyManager {
  fn drop(&mut self) {
    if let Err(err) = self.unregister_all() {
      error!("drop: failed to unregister all hotkeys {:?}", err);
    }
  }
}

static REGEX_HOTKEY_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
  regex::Regex::new(
        r"^(?i)(?:(CTRL|SHIFT|ALT|SUPER)\+){0,1}(?:(CTRL|SHIFT|ALT|SUPER)\+){0,1}(?:(CTRL|SHIFT|ALT|SUPER)\+){0,1}(?:(CTRL|SHIFT|ALT|SUPER)\+){0,1}(\w+)$",
    ).unwrap()
});

pub fn parse_hotkey(hotkey_string: &str) -> Result<Hotkey> {
  let caps: regex::Captures = REGEX_HOTKEY_PATTERN
    .captures(hotkey_string)
    .ok_or_else(|| Error::InvalidHotkey("regex dind't match".to_string()))?;
  let mut modifier = Vec::new();
  let mut key: Option<Key> = None;
  for caps in caps.iter().skip(1) {
    if let Some(caps) = caps {
      let mut mat = caps.as_str().to_uppercase();
      if mat.parse::<usize>().is_ok() {
        mat = format!("KEY_{}", mat);
      }
      if let Ok(res) = Modifier::from_str(&mat) {
        modifier.push(res);
        continue;
      }
      if key.is_some() {
        return Err(Error::InvalidHotkey(
          "hotkey has alread a key specified".to_string(),
        ));
      }
      if let Ok(res) = Key::from_str(&mat) {
        key = Some(res);
      }
    }
  }

  match key {
    Some(key) => Ok(Hotkey { modifier, key }),
    None => Err(Error::InvalidHotkey(
      "hotkey has no key specified".to_string(),
    )),
  }
}

#[derive(Debug, Deserialize, Clone, Serialize, PartialEq, Hash, Eq)]
pub struct Hotkey {
  pub modifier: Vec<Modifier>,
  pub key: Key,
}

impl Hotkey {
  pub fn modifier_as_flag(&self) -> u32 {
    self.modifier.iter().fold(0, |acc, x| acc | (*x as u32)) as u32
  }
}

#[derive(
  Debug, Deserialize, Copy, Clone, Serialize, strum_macros::EnumString, PartialEq, Hash, Eq,
)]
#[repr(u32)]
pub enum Modifier {
  ALT = modifiers::ALT,
  CTRL = modifiers::CONTROL,
  SHIFT = modifiers::SHIFT,
  SUPER = modifiers::SUPER,
}

impl fmt::Display for Modifier {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

#[allow(non_camel_case_types)]
#[derive(
  Debug, Deserialize, Copy, Clone, Serialize, strum_macros::EnumString, PartialEq, Hash, Eq,
)]
#[repr(u32)]
pub enum Key {
  BACKSPACE = keys::BACKSPACE,
  TAB = keys::TAB,
  ENTER = keys::ENTER,
  CAPS_LOCK = keys::CAPS_LOCK,
  ESCAPE = keys::ESCAPE,
  SPACEBAR = keys::SPACEBAR,
  PAGE_UP = keys::PAGE_UP,
  PAGE_DOWN = keys::PAGE_DOWN,
  END = keys::END,
  HOME = keys::HOME,
  ARROW_LEFT = keys::ARROW_LEFT,
  ARROW_RIGHT = keys::ARROW_RIGHT,
  ARROW_UP = keys::ARROW_UP,
  ARROW_DOWN = keys::ARROW_DOWN,
  PRINT_SCREEN = keys::PRINT_SCREEN,
  INSERT = keys::INSERT,
  DELETE = keys::DELETE,
  #[serde(rename = "0")]
  KEY_0 = keys::KEY_0,
  #[serde(rename = "1")]
  KEY_1 = keys::KEY_1,
  #[serde(rename = "2")]
  KEY_2 = keys::KEY_2,
  #[serde(rename = "3")]
  KEY_3 = keys::KEY_3,
  #[serde(rename = "4")]
  KEY_4 = keys::KEY_4,
  #[serde(rename = "5")]
  KEY_5 = keys::KEY_5,
  #[serde(rename = "6")]
  KEY_6 = keys::KEY_6,
  #[serde(rename = "7")]
  KEY_7 = keys::KEY_7,
  #[serde(rename = "8")]
  KEY_8 = keys::KEY_8,
  #[serde(rename = "9")]
  KEY_9 = keys::KEY_9,
  A = keys::A,
  B = keys::B,
  C = keys::C,
  D = keys::D,
  E = keys::E,
  F = keys::F,
  G = keys::G,
  H = keys::H,
  I = keys::I,
  J = keys::J,
  K = keys::K,
  L = keys::L,
  M = keys::M,
  N = keys::N,
  O = keys::O,
  P = keys::P,
  Q = keys::Q,
  R = keys::R,
  S = keys::S,
  T = keys::T,
  U = keys::U,
  V = keys::V,
  W = keys::W,
  X = keys::X,
  Y = keys::Y,
  Z = keys::Z,
}

impl fmt::Display for Key {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl fmt::Display for Hotkey {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let modifier_string: String = self.modifier.iter().fold(String::new(), |all, one| {
      if !all.is_empty() {
        format!("{}-{}", all, one)
      } else {
        one.to_string()
      }
    });
    let hotkey_string = {
      if !modifier_string.is_empty() {
        format!("{}-{}", modifier_string, self.key.to_string())
      } else {
        self.key.to_string()
      }
    };
    write!(f, "{}", hotkey_string)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hotkey_parse() {
    assert_eq!(
      parse_hotkey("CTRL-P").unwrap(),
      Hotkey {
        modifier: vec![Modifier::CTRL],
        key: Key::P
      }
    );
    assert_eq!(
      parse_hotkey("CTRL-SHIFT-P").unwrap(),
      Hotkey {
        modifier: vec![Modifier::CTRL, Modifier::SHIFT],
        key: Key::P
      }
    );
    assert_eq!(
      parse_hotkey("S").unwrap(),
      Hotkey {
        modifier: vec![],
        key: Key::S
      }
    );
    assert_eq!(
      parse_hotkey("ALT-BACKSPACE").unwrap(),
      Hotkey {
        modifier: vec![Modifier::ALT],
        key: Key::BACKSPACE
      }
    );
    assert_eq!(
      parse_hotkey("SHIFT-SUPER-A").unwrap(),
      Hotkey {
        modifier: vec![Modifier::SHIFT, Modifier::SUPER],
        key: Key::A
      }
    );
    assert_eq!(
      parse_hotkey("SUPER-ARROW_RIGHT").unwrap(),
      Hotkey {
        modifier: vec![Modifier::SUPER],
        key: Key::ARROW_RIGHT
      }
    );
    assert_eq!(
      parse_hotkey("SUPER-CTRL-SHIFT-ALT-9").unwrap(),
      Hotkey {
        modifier: vec![
          Modifier::SUPER,
          Modifier::CTRL,
          Modifier::SHIFT,
          Modifier::ALT
        ],
        key: Key::KEY_9
      }
    );
    assert_eq!(
      parse_hotkey("super-ctrl-SHIFT-alt-ARROW_Up").unwrap(),
      Hotkey {
        modifier: vec![
          Modifier::SUPER,
          Modifier::CTRL,
          Modifier::SHIFT,
          Modifier::ALT
        ],
        key: Key::ARROW_UP
      }
    );

    assert_eq!(
      parse_hotkey("5").unwrap(),
      Hotkey {
        modifier: vec![],
        key: Key::KEY_5
      }
    );

    assert_eq!(
      parse_hotkey("KEY_5").unwrap(),
      Hotkey {
        modifier: vec![],
        key: Key::KEY_5
      }
    );

    assert_eq!(
      parse_hotkey("5-5").unwrap_err().to_string(),
      "No valid hotkey match"
    );

    assert_eq!(
      parse_hotkey("CTRL-").unwrap_err().to_string(),
      "No valid hotkey match"
    );

    assert_eq!(
      parse_hotkey("").unwrap_err().to_string(),
      "No valid hotkey match"
    );
  }
}
