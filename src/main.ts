import { mount } from 'svelte';
import './lib/global.css';
import App from './App.svelte';

mount(App, { target: document.getElementById('app')! });