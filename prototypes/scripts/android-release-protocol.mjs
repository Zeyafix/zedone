import assert from "node:assert/strict";
import { inflateSync } from "node:zlib";

// JSON line followed, only for RGBA screenshots, by an explicitly sized body.
// TCP/ADB may split either part anywhere, including inside a UTF-8 character.
export function responseDecoder(receive) {
  let buffered = Buffer.alloc(0), header;
  return chunk => {
    buffered = Buffer.concat([buffered, chunk]);
    if (!header) {
      const end = buffered.indexOf(10);
      assert.ok((end < 0 ? buffered.length : end) <= 16 * 1024 * 1024, "Release response header too large");
      if (end < 0) return;
      header = JSON.parse(buffered.subarray(0, end).toString("utf8"));
      buffered = buffered.subarray(end + 1);
      if (header.ok && ["rgba8", "rgba8-zlib"].includes(header.value?.format)) {
        const { width, height, byte_length } = header.value;
        assert.ok(Number.isSafeInteger(width) && width > 0 && Number.isSafeInteger(height) && height > 0);
        assert.ok(Number.isSafeInteger(byte_length) && byte_length > 0 && byte_length <= 64 * 1024 * 1024);
        assert.equal(byte_length, width * height * 4, "Invalid RGBA frame length");
        if (header.value.format === "rgba8-zlib") {
          const size = header.value.wire_byte_length;
          assert.ok(Number.isSafeInteger(size) && size > 0 && size <= 65 * 1024 * 1024, "Invalid compressed frame length");
        }
      }
    }
    const size = !header.ok ? 0 : header.value?.format === "rgba8-zlib" ? header.value.wire_byte_length : header.value?.format === "rgba8" ? header.value.byte_length : 0;
    if (buffered.length < size) return;
    assert.equal(buffered.length, size, "Unexpected bytes after release response");
    const response = header;
    if (size) {
      if (response.value.format === "rgba8-zlib") {
        const decoded = inflateSync(buffered, { maxOutputLength: response.value.byte_length, info: true });
        assert.equal(decoded.engine.bytesWritten, buffered.length, "Trailing compressed data");
        assert.equal(decoded.buffer.length, response.value.byte_length, "Decoded RGBA length mismatch");
        response.value.rgba = decoded.buffer;
        response.value.compressed = buffered;
      } else response.value.rgba = buffered;
    }
    header = undefined; buffered = Buffer.alloc(0);
    receive(response);
  };
}
