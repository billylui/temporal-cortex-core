import { describe, it, expect } from "vitest";
import { encode, decode } from "../src/index.js";

describe("encode", () => {
  it("encodes a flat object", () => {
    const json = '{"name":"Alice","age":30}';
    const toon = encode(json);
    expect(toon).toBe("name: Alice\nage: 30");
  });

  it("encodes nested objects", () => {
    const json = '{"server":{"host":"localhost","port":8080}}';
    const toon = encode(json);
    expect(toon).toBe("server:\n  host: localhost\n  port: 8080");
  });

  it("encodes inline arrays", () => {
    const json = '{"ids":[1,2,3]}';
    const toon = encode(json);
    expect(toon).toBe("ids[3]: 1,2,3");
  });

  it("encodes tabular arrays", () => {
    const json =
      '{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}';
    const toon = encode(json);
    expect(toon).toBe("users[2]{id,name}:\n  1,Alice\n  2,Bob");
  });

  it("encodes primitives", () => {
    expect(encode("null")).toBe("null");
    expect(encode("true")).toBe("true");
    expect(encode("42")).toBe("42");
    expect(encode('"hello"')).toBe("hello");
  });

  it("quotes strings that look like keywords", () => {
    const json = '{"val":"true"}';
    const toon = encode(json);
    expect(toon).toBe('val: "true"');
  });

  it("handles empty objects and arrays", () => {
    expect(encode("{}")).toBe("");
    expect(encode('{"items":[]}')).toBe("items[0]:");
  });
});

describe("decode", () => {
  it("decodes a flat object", () => {
    const toon = "name: Alice\nage: 30";
    const json = JSON.parse(decode(toon));
    expect(json).toEqual({ name: "Alice", age: 30 });
  });

  it("decodes nested objects", () => {
    const toon = "server:\n  host: localhost\n  port: 8080";
    const json = JSON.parse(decode(toon));
    expect(json).toEqual({ server: { host: "localhost", port: 8080 } });
  });

  it("decodes inline arrays", () => {
    const toon = "ids[3]: 1,2,3";
    const json = JSON.parse(decode(toon));
    expect(json).toEqual({ ids: [1, 2, 3] });
  });

  it("decodes tabular arrays", () => {
    const toon = "users[2]{id,name}:\n  1,Alice\n  2,Bob";
    const json = JSON.parse(decode(toon));
    expect(json).toEqual({
      users: [
        { id: 1, name: "Alice" },
        { id: 2, name: "Bob" },
      ],
    });
  });

  it("decodes primitives", () => {
    expect(JSON.parse(decode("null"))).toBe(null);
    expect(JSON.parse(decode("true"))).toBe(true);
    expect(JSON.parse(decode("42"))).toBe(42);
    expect(JSON.parse(decode("hello"))).toBe("hello");
  });

  it("decodes quoted values as strings", () => {
    const toon = 'val: "42"';
    const json = JSON.parse(decode(toon));
    expect(json).toEqual({ val: "42" });
  });

  it("decodes empty input as empty object", () => {
    const json = JSON.parse(decode(""));
    expect(json).toEqual({});
  });
});

describe("roundtrip", () => {
  const cases = [
    "null",
    "true",
    "42",
    '"hello"',
    "{}",
    '{"a":1}',
    '{"name":"Alice","age":30}',
    '{"server":{"host":"localhost","port":8080}}',
    '{"ids":[1,2,3]}',
    '{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}',
    '{"items":["hello",42,true,null]}',
    '[1,2,3]',
  ];

  for (const input of cases) {
    it(`roundtrips: ${input.slice(0, 40)}`, () => {
      const toon = encode(input);
      const output = decode(toon);
      expect(JSON.parse(output)).toEqual(JSON.parse(input));
    });
  }
});
