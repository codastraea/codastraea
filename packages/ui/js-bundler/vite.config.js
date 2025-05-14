import { resolve } from 'path';
import { defineConfig } from 'vite';
import { viteStaticCopy } from 'vite-plugin-static-copy';

const shoelace_dist = resolve(__dirname, 'node_modules/@shoelace-style/shoelace/dist');

module.exports = defineConfig({
    build: {
        lib: {
            entry: resolve(__dirname, 'main.js'),
            name: 'SerpentAutomation',
            formats: ['esm'],
        },
    },
    plugins: [
        viteStaticCopy({
            targets: [
                {
                    src: resolve(shoelace_dist, 'themes/*.css'),
                    dest: 'shoelace/themes'
                }
                , {
                    src: resolve(shoelace_dist, 'assets'),
                    dest: 'shoelace'
                }
            ]
        })
    ],
})
