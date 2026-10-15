# 09 — Autostart (Start with Windows)

## 1. Overview

PinIt can start automatically when the user logs in to Windows. This is implemented using the Windows registry Run key — the standard mechanism for per-user autostart programs. When enabled, PinIt starts minimised to the tray so it does not disrupt the login experience.

---

## 2. The Registry Key

**Key path:** `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`

**Value name:** `PinIt`

**Value type:** `REG_SZ` (string)

**Value data format:** `"<full path to PinIt.exe>" --minimized`

Example:
```
"C:\Users\Alice\AppData\Local\Programs\PinIt\PinIt.exe" --minimized
```

The path is enclosed in double quotes to handle paths with spaces. The `--minimized` flag tells PinIt to start in tray mode (hidden window).

### 2.1 Per-user, no admin required

This is the `HKCU` hive, not `HKLM`. It applies only to the current user and does not require administrator privilege. PinIt should never attempt to write to `HKLM\...\Run` — that would require elevation and is wrong for a per-user utility.

### 2.2 Dev mode isolation

In dev mode (`PINIT_DEV=1`), use the value name `"PinIt-dev"` instead of `"PinIt"`. This prevents the dev build's autostart entry from conflicting with an installed release copy's entry.

---

## 3. The --minimized Flag

When PinIt is launched with `--minimized` as a command-line argument:
- Do not show the main window on startup.
- Add the tray icon.
- Restore saved pins silently in the background.

Without this flag (normal launch), the main window is shown on startup.

Parsing: check `std::env::args()` (Rust) or `process.argv` (Node/Tauri frontend) for the string `"--minimized"`. It can appear anywhere in the argument list.

---

## 4. Checking Autostart State

To determine whether autostart is currently enabled, read the registry value:

Procedure:
1. Open `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
2. Read the `"PinIt"` value.
3. If the value exists and is a non-empty string → autostart is enabled.
4. If the value does not exist → autostart is disabled.

The return value of this check is what the settings toggle reflects. Always read from the registry — do not rely on the `start_with_windows` field in `pinned.json` for the actual state. That field is for display purposes only (to initialise the toggle without a registry read).

---

## 5. Enabling Autostart

When the user enables the autostart toggle:

1. Get the full path to the current running executable.
   - In Rust: `std::env::current_exe()`.
   - In Tauri: available via the app handle.
2. Build the value string: `"\"<path>\" --minimized"`.
3. Write the value to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` under the name `"PinIt"`.
4. If the write succeeds, update the in-memory settings and save to `pinned.json`.
5. If the write fails, show an error in the UI: "Couldn't change the startup setting."

---

## 6. Disabling Autostart

When the user disables the autostart toggle:

1. Delete the `"PinIt"` value from `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
2. If the value does not exist, treat as success (it was already disabled).
3. If the delete fails, show an error.

---

## 7. Path Repair

The `repairPath` function handles the case where autostart is enabled but the registered path is stale — for example, after the user moved PinIt from one folder to another (portable use case).

Procedure:

1. Read the current value from the registry.
2. If it does not exist → nothing to repair, return `false`.
3. Extract the executable path from the value (strip the `--minimized` argument and the surrounding quotes).
4. Check whether the extracted path points to the current running executable:
   - If they match → already correct, return `false` (no repair needed).
5. Check whether the extracted path points to a file that exists on disk:
   - If it exists but is a different copy → another installation owns the entry. Do not overwrite it. Return `false`.
   - If it does NOT exist → the registered path is stale. Overwrite with the current path. Return `true` (repair was performed).

The repair function is called once on startup. If it returns `true`, log a warning: `"Repaired autostart registry path"`.

---

## 8. Interaction with the Installer

The Inno Setup installer offers autostart as an optional task during installation. When the user checks the "Start PinIt automatically when Windows starts" option in the installer, the installer writes the same `HKCU\...\Run` entry with the same value name (`"PinIt"`) and the same `--minimized` flag.

This means the installer and the in-app toggle share the same registry value. They always agree. If the user enables autostart from the installer and then opens PinIt's settings, the toggle should already show as enabled (because the registry check finds the value).

---

## 9. Uninstall Cleanup

The installer's `[Registry]` section includes an entry with `Flags: uninsdeletevalue` that removes the `"PinIt"` value from the Run key on uninstall. This covers both cases:
- Autostart was set by the installer task.
- Autostart was set by the user from within the app.

After uninstalling, no Run key entry should remain pointing to the deleted PinIt.exe.

---

## 10. Testing Autostart

In tests, the autostart module must not touch the real `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` key. The module should accept an override key path for testing — for example, `HKEY_CURRENT_USER\Software\PinIt-Tests\Run-<pid>`. The test can then create, modify, and delete values in this scratch key without risk, and clean it up after itself.

This is exactly how the C++ test suite works: it calls a `setRegistryKeyForTesting()` function that redirects all autostart reads and writes to a scratch key. The same pattern should be used in the Tauri rebuild.
