import { mount } from 'svelte';
import App from './LiveApp.svelte';
import 'maplibre-gl/dist/maplibre-gl.css';
mount(App, { target: document.getElementById('app')! });
