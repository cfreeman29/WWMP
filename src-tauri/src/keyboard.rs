use anyhow::{anyhow, Result};

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_LSHIFT, VK_LCONTROL,
    VK_Q, VK_W, VK_E, VK_R, VK_T, VK_Y, VK_U,
    VK_A, VK_S, VK_D, VK_F, VK_G, VK_H, VK_J,
    VK_Z, VK_X, VK_C, VK_V, VK_B, VK_N, VK_M,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    None,
    Shift,  // Sharp
    Ctrl,   // Flat
}

/// Convert a key string to a virtual key code
#[cfg(windows)]
fn key_to_vk(key: &str) -> Result<VIRTUAL_KEY> {
    match key.to_uppercase().as_str() {
        "Q" => Ok(VK_Q),
        "W" => Ok(VK_W),
        "E" => Ok(VK_E),
        "R" => Ok(VK_R),
        "T" => Ok(VK_T),
        "Y" => Ok(VK_Y),
        "U" => Ok(VK_U),
        "A" => Ok(VK_A),
        "S" => Ok(VK_S),
        "D" => Ok(VK_D),
        "F" => Ok(VK_F),
        "G" => Ok(VK_G),
        "H" => Ok(VK_H),
        "J" => Ok(VK_J),
        "Z" => Ok(VK_Z),
        "X" => Ok(VK_X),
        "C" => Ok(VK_C),
        "V" => Ok(VK_V),
        "B" => Ok(VK_B),
        "N" => Ok(VK_N),
        "M" => Ok(VK_M),
        _ => Err(anyhow!("Unknown key: {}", key)),
    }
}

#[cfg(windows)]
fn modifier_to_vk(modifier: Modifier) -> Option<VIRTUAL_KEY> {
    match modifier {
        Modifier::None => None,
        Modifier::Shift => Some(VK_LSHIFT),
        Modifier::Ctrl => Some(VK_LCONTROL),
    }
}

#[cfg(windows)]
fn create_key_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
fn send_inputs(inputs: &[INPUT]) -> Result<()> {
    let sent = unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32)
    };
    if sent != inputs.len() as u32 {
        return Err(anyhow!("SendInput failed: sent {} of {}", sent, inputs.len()));
    }
    Ok(())
}

/// Press a key with optional modifier
#[cfg(windows)]
pub fn press_key(key: &str, modifier: Modifier) -> Result<()> {
    let vk = key_to_vk(key)?;
    let mut inputs = Vec::new();

    // Press modifier first if needed
    if let Some(mod_vk) = modifier_to_vk(modifier) {
        inputs.push(create_key_input(mod_vk, false));
    }

    // Press the main key
    inputs.push(create_key_input(vk, false));

    send_inputs(&inputs)
}

/// Release a key with optional modifier
#[cfg(windows)]
pub fn release_key(key: &str, modifier: Modifier) -> Result<()> {
    let vk = key_to_vk(key)?;
    let mut inputs = Vec::new();

    // Release main key first
    inputs.push(create_key_input(vk, true));

    // Release modifier if needed
    if let Some(mod_vk) = modifier_to_vk(modifier) {
        inputs.push(create_key_input(mod_vk, true));
    }

    send_inputs(&inputs)
}

/// Release all keys (panic button)
#[cfg(windows)]
pub fn release_all() -> Result<()> {
    let all_keys = [
        VK_Q, VK_W, VK_E, VK_R, VK_T, VK_Y, VK_U,
        VK_A, VK_S, VK_D, VK_F, VK_G, VK_H, VK_J,
        VK_Z, VK_X, VK_C, VK_V, VK_B, VK_N, VK_M,
        VK_LSHIFT, VK_LCONTROL,
    ];

    let inputs: Vec<INPUT> = all_keys
        .iter()
        .map(|&vk| create_key_input(vk, true))
        .collect();

    send_inputs(&inputs)
}

// Non-Windows stubs for development
#[cfg(not(windows))]
pub fn press_key(key: &str, modifier: Modifier) -> Result<()> {
    println!("STUB: press_key({}, {:?})", key, modifier);
    Ok(())
}

#[cfg(not(windows))]
pub fn release_key(key: &str, modifier: Modifier) -> Result<()> {
    println!("STUB: release_key({}, {:?})", key, modifier);
    Ok(())
}

#[cfg(not(windows))]
pub fn release_all() -> Result<()> {
    println!("STUB: release_all()");
    Ok(())
}
