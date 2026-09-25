//! Точка входа бинаря. Только атрибут Windows-подсистемы + main().
//! Binary entry point. Only the Windows subsystem attribute + main().
//! НЕ УДАЛЯТЬ первую строку — иначе на Windows откроется лишняя консоль.
//! DO NOT REMOVE line 1 — otherwise Windows spawns an extra console window.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    compound_lib::run()
}