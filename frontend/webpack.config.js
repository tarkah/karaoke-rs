const path = require('path');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');

module.exports = (env, argv) => {
  return {
    devServer: {
      historyApiFallback: true,
      compress: argv.mode === 'production',
      port: 8000,
      host: '0.0.0.0',
      proxy: {
        '/api': 'http://127.0.0.1:8080',
      },
    },
    entry: './bootstrap.js',
    output: {
      path: path.resolve(__dirname, "./dist"),
      filename: "karaoke-rs.js",
      webassemblyModuleFilename: "karaoke-rs.wasm",
    },
    plugins: [
      process.env.NODE_ENV === 'production' ? new CleanWebpackPlugin() : false,
      new CopyWebpackPlugin([
        { from: './static', to: path.resolve(__dirname, "./dist") }
      ]),
      new WasmPackPlugin({
        crateDirectory: ".",
        extraArgs: "--no-typescript",
      })
    ].filter(Boolean),
    watch: argv.mode !== 'production'
  };
};