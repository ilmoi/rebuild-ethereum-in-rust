// const keccak256 = require('js-sha3').keccak256;
// const EC = require('elliptic').ec;
//
// const ec = new EC('secp256k1');
//
// const sortCharacters = data => {
//   return JSON.stringify(data).split('').sort().join('');
// }
//
// //npm i js-sha3
// const keccakHash = data => {
//   const hash = keccak256.create();
//   //seed the object with the data that we want to produce a hash for
//   //sort chars takes out any reliance on order of keys in the object
//   hash.update(sortCharacters(data));
//   //return as hex
//   return hash.hex();
// }
//
// module.exports = {
//   sortCharacters,
//   keccakHash,
//   ec
// };