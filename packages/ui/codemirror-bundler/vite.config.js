const path = require('path')
const { defineConfig } = require('vite')

module.exports = defineConfig({
    // TODO: This is a workaround to enable minification for ES builds.
    //
    // One of the proposed solutions to minifying ES builds is to default
    // `build.minify`to false, then allow us to opt-in.
    //
    // See:
    // - <https://github.com/vitejs/vite/issues/6555>
    // - <https://github.com/vitejs/vite/pull/6670>
    esbuild: {
        minify: true,
    },
    build: {
        minify: true,
        lib: {
            entry: path.resolve(__dirname, 'main.js'),
            name: 'CodeMirror',
            fileName: (format) => `codemirror.${format}.js`,
            formats: ['esm'],
        },
    }
})
