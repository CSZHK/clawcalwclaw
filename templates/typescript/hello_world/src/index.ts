/**
 * __SKILL_NAME__ — clawclawclaw Skill (TypeScript)
 *
 * Protocol: read JSON from stdin, write JSON result to stdout.
 * Build:    npm install && npm run build  →  tool.wasm
 * Requires: javy CLI  →  https://github.com/bytecodealliance/javy
 * Test:     clawclawclaw skill test . --args '{"name":"clawclawclaw"}'
 */

interface Args {
  name: string;
}

interface ToolResult {
  success: boolean;
  output: string;
  error?: string;
}

function run(args: Args): ToolResult {
  const greeting = `Hello, ${args.name}! Welcome to clawclawclaw skills.`;
  return { success: true, output: greeting };
}

let result: ToolResult;
try {
  // @ts-ignore — Javy provides synchronous IO
  const rawInput = new TextDecoder().decode(Javy.IO.readSync());
  const input = JSON.parse(rawInput);
  if (!input.name) throw new Error('missing required field: name');
  result = run(input as Args);
} catch (e: unknown) {
  result = { success: false, output: '', error: String(e) };
}

// @ts-ignore
Javy.IO.writeSync(new TextEncoder().encode(JSON.stringify(result)));
