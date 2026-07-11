import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { exec } from 'child_process'

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    {
      name: 'open-file-server',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url && req.url.startsWith('/api/open-file')) {
            const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);
            const filePath = url.searchParams.get('path');
            if (filePath) {
              exec(`open "${filePath}"`, (error) => {
                if (error) {
                  res.statusCode = 500;
                  res.end(JSON.stringify({ error: error.message }));
                } else {
                  res.statusCode = 200;
                  res.end(JSON.stringify({ success: true }));
                }
              });
            } else {
              res.statusCode = 400;
              res.end(JSON.stringify({ error: 'Missing path' }));
            }
          } else {
            next();
          }
        });
      }
    }
  ],
  server: {
    host: true
  }
})