// Типы для .css и xterm — их нет в @types, поэтому объявляем руками.
// Types for .css and xterm — no @types package, so we declare them by hand.
// Нужно VS/TS, чтобы импортировать стили и терминал без ошибок.
// Lets TS/Vite import styles and the terminal without errors.
/// <reference types="svelte" />
/// <reference types="vite/client" />

declare module '*.css';
declare module '@xterm/xterm';
declare module '@xterm/addon-fit';