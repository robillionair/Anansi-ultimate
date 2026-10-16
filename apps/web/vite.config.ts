import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({plugins:[react()],server:{host:'127.0.0.1',port:1420,strictPort:true},preview:{headers:{'Content-Security-Policy':"default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src http://127.0.0.1:*; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"}},build:{target:'es2022'}});
