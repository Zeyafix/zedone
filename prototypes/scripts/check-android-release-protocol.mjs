import assert from "node:assert/strict";
import { deflateSync } from "node:zlib";
import { randomBytes } from "node:crypto";
import { responseDecoder } from "./android-release-protocol.mjs";

// ADB/TCP chunks have no relationship to JSON lines, UTF-8 or pixel boundaries.
const rgba = Buffer.from([0, 10, 255, 255, 13, 123, 128, 0]);
const frame = Buffer.concat([Buffer.from(JSON.stringify({ ok: true, value: {
  format: "rgba8", width: 2, height: 1, byte_length: 8,
} }) + "\n"), rgba]);
for (let split = 0; split <= frame.length; split++) {
  const responses = [], decode = responseDecoder(value => responses.push(value));
  if (split) decode(frame.subarray(0, split));
  if (split < frame.length) {
    assert.equal(responses.length, 0);
    decode(frame.subarray(split));
  }
  assert.equal(responses.length, 1);
  assert.deepEqual(responses[0].value.rgba, rgba);
  // The next response follows the binary body without a separator.
  const next = Buffer.from(JSON.stringify({ ok: true, value: "Карточка 🦊" }) + "\n");
  for (const byte of next) decode(Buffer.from([byte]));
  assert.equal(responses[1].value, "Карточка 🦊");
}
for (const fields of [
  { width: 2, height: 1, byte_length: 7 },
  { width: 0, height: 1, byte_length: 0 },
  { width: 1.5, height: 1, byte_length: 6 },
  { width: 4096, height: 4097, byte_length: 4096 * 4097 * 4 },
]) {
  assert.throws(() => responseDecoder(() => assert.fail("Invalid frame accepted"))(
    Buffer.from(JSON.stringify({ ok: true, value: { format: "rgba8", ...fields } }) + "\n")));
}
assert.throws(() => responseDecoder(() => {})(Buffer.from("invalid JSON\n")));
assert.throws(() => responseDecoder(() => {})(Buffer.concat([frame, Buffer.from([0])])));
const responses = [], decode = responseDecoder(response => responses.push(response));
decode(Buffer.from('{"ok":false,"error":"failed"}\n'));
decode(Buffer.from('{"ok":true,"value":{"png":"legacy"}}\n'));
assert.deepEqual(responses, [{ ok: false, error: "failed" }, { ok: true, value: { png: "legacy" } }]);
console.log("Release protocol: split JSON/UTF-8/RGBA, byte order, framing limits and legacy replies passed.");

const compressedFrame = (body, fields = {}) => Buffer.concat([Buffer.from(JSON.stringify({ ok: true, value: {
  format: "rgba8-zlib", width: 2, height: 1, byte_length: 8, wire_byte_length: body.length, ...fields,
} }) + "\n"), body]);
const compressed = deflateSync(rgba, { level: 1 });
const zipped = compressedFrame(compressed);
for (let split = 0; split <= zipped.length; split++) {
  const replies = [], consume = responseDecoder(value => replies.push(value));
  if (split) consume(zipped.subarray(0, split));
  if (split < zipped.length) { assert.equal(replies.length, 0); consume(zipped.subarray(split)); }
  assert.equal(replies.length, 1);
  assert.deepEqual(replies[0].value.rgba, rgba);
  assert.deepEqual(replies[0].value.compressed, compressed);
  consume(frame);
  assert.deepEqual(replies[1].value.rgba, rgba);
}
// Incompressible data can grow; compressed length need not be <= RGBA length.
for (const data of [randomBytes(65536), Buffer.alloc(65536)]) {
  const reply = [];
  const bytes = compressedFrame(deflateSync(data, { level: 1 }), { width: 128, height: 128, byte_length: data.length });
  const consume = responseDecoder(value => reply.push(value));
  for (const byte of bytes) consume(Buffer.from([byte]));
  assert.deepEqual(reply[0].value.rgba, data);
}
for (const fields of [
  { byte_length: 7 }, { width: -1 }, { byte_length: 67108868, width: 1, height: 16777217 },
  { wire_byte_length: 0 }, { wire_byte_length: -1 }, { wire_byte_length: 1.5 },
  { wire_byte_length: 65 * 1024 * 1024 + 1 }, { wire_byte_length: null },
]) assert.throws(() => responseDecoder(() => assert.fail("Invalid compressed frame accepted"))(compressedFrame(compressed, fields)));
const corrupt = Buffer.from(compressed); corrupt[corrupt.length - 1] ^= 1;
for (const body of [
  corrupt, compressed.subarray(0, -1), Buffer.from("invalid zlib"),
  Buffer.concat([compressed, Buffer.from([0])]), Buffer.concat([compressed, compressed]),
  deflateSync(Buffer.alloc(9)), deflateSync(Buffer.alloc(7)),
]) assert.throws(() => responseDecoder(() => assert.fail("Invalid zlib accepted"))(compressedFrame(body)));
assert.throws(() => responseDecoder(() => {})(Buffer.concat([zipped, Buffer.from([0])])));
console.log("Release zlib: arbitrary splits, exact lossless bytes, growing payload, size limits, checksum/truncation/trailing data and bounded inflation passed.");
