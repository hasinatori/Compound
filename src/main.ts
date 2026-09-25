// Точка входа: монтируем App в #app. Тут ничего больше.
// Entry point: mount App into #app. Nothing else here.
import { mount } from 'svelte';
import './lib/global.css';
import App from './App.svelte';

mount(App, { target: document.getElementById('app')! });