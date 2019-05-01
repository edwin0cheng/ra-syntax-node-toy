// ./index.js
const express = require('express');
const path = require('path');

const app = express();

const port = process.env.PORT || 3000;

express.static.mime.types["wasm"] = "application/wasm";

app.use(express.static('dist'));

app.listen(port, () => {
  console.log(`App listening on port ${port}`)
});