const input = require('fs').readFileSync(0, 'utf8').trim();
console.log(Buffer.from(input, 'base64').toString());
