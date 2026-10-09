import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
const proxy = process.env.ROM_LIVE_FIXTURE === '1' ? Object.fromEntries(['/points','/selection','/geocode','/fixture'].map(path => [path,{ target: 'http://127.0.0.1:55468', changeOrigin: false }])) : undefined;
export default defineConfig({ plugins: [svelte()], server: { host: '127.0.0.1', proxy }, preview: { proxy } });
