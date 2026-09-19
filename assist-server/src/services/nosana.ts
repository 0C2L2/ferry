import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";

// Nosana's officially documented interface for posting an inference job is
// the CLI (`nosana job post ...`) rather than a stable hosted SDK method, so
// we shell out to it instead of guessing at an API surface that may not
// match what's actually deployed. The job definition (which Docker image
// runs, what port it serves an OpenAI-compatible API on) is exported from
// the Nosana dashboard's "Deploy" flow for whichever model template is
// picked — see assist-server/README.md.
let cachedEndpoint: string | null = null;
let deployPromise: Promise<string> | null = null;

async function deploy(): Promise<string> {
  const jobDefPath = process.env.NOSANA_JOB_DEFINITION_PATH ?? "./nosana-job-definition.json";
  const market = process.env.NOSANA_MARKET;
  if (!market) throw new Error("NOSANA_MARKET is not set");
  await readFile(jobDefPath, "utf-8"); // fail fast with a clear error if the job definition is missing

  return new Promise<string>((resolve, reject) => {
    const proc = spawn("nosana", ["job", "post", jobDefPath, "--market", market, "--wait"], {
      shell: true,
    });
    let out = "";
    proc.stdout.on("data", d => (out += d.toString()));
    proc.stderr.on("data", d => (out += d.toString()));
    proc.on("error", reject);
    proc.on("close", code => {
      if (code !== 0) return reject(new Error(`nosana job post failed:\n${out}`));
      const match = out.match(/https:\/\/[a-zA-Z0-9.-]+\.node\.k8s\.[a-zA-Z0-9.-]+/);
      if (!match) return reject(new Error(`Could not find an exposed endpoint in Nosana output:\n${out}`));
      resolve(match[0]);
    });
  });
}

async function getEndpoint(): Promise<string> {
  if (cachedEndpoint) return cachedEndpoint;
  if (!deployPromise) {
    deployPromise = deploy().then(url => {
      cachedEndpoint = url;
      return url;
    });
  }
  return deployPromise;
}

export interface AppSuggestionRequest {
  name: string;
  publisher: string | null;
  tier: number;
}

export interface AppSuggestion {
  name: string;
  suggestion: string;
  command: string | null;
}

const SYSTEM_PROMPT = `You help a non-technical person who is migrating their files and apps to a
new operating system. You will be given a list of installed apps that could not be
matched to a confident, verified reinstall source. For each app, in one plain-language
sentence, explain what it likely is and whether the user probably needs it on the new
system. If the target OS differs from Windows, suggest a well-known cross-platform or
native equivalent when one exists.

Respond with ONLY a JSON array, no prose, of objects shaped exactly like:
{"name": string, "suggestion": string, "command": string | null}

Only set "command" when you are confident it is a single, safe, standard package-manager
install command (e.g. "winget install Publisher.App" or "flatpak install flathub org.app.App").
Never suggest a command that deletes, formats, modifies permissions, or downloads from an
unverified URL. When unsure, leave "command" null.`;

function buildPrompt(apps: AppSuggestionRequest[], targetFamily: string): string {
  const list = apps
    .map(a => `- ${a.name}${a.publisher ? ` (${a.publisher})` : ""} [tier ${a.tier}]`)
    .join("\n");
  return `Target OS family: ${targetFamily}\n\nApps with no confident reinstall source:\n${list}`;
}

function extractJsonArray(text: string): string {
  const start = text.indexOf("[");
  const end = text.lastIndexOf("]");
  if (start === -1 || end === -1 || end < start) throw new Error("no JSON array found in model output");
  return text.slice(start, end + 1);
}

function parseSuggestions(text: string, apps: AppSuggestionRequest[]): AppSuggestion[] {
  try {
    const parsed = JSON.parse(extractJsonArray(text));
    if (Array.isArray(parsed)) {
      return parsed.map((entry: Partial<AppSuggestion>, i: number) => ({
        name: typeof entry.name === "string" ? entry.name : apps[i]?.name ?? "unknown",
        suggestion: typeof entry.suggestion === "string" ? entry.suggestion : "No suggestion available.",
        command: typeof entry.command === "string" ? entry.command : null,
      }));
    }
  } catch {
    // Model didn't return clean JSON — fall back below rather than crash the request.
  }
  return apps.map(a => ({ name: a.name, suggestion: text.trim() || "No suggestion available.", command: null }));
}

export async function suggestForApps(
  apps: AppSuggestionRequest[],
  targetFamily: string,
): Promise<AppSuggestion[]> {
  const endpoint = await getEndpoint();
  const res = await fetch(`${endpoint}/v1/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      model: process.env.NOSANA_MODEL ?? "default",
      temperature: 0.2,
      messages: [
        { role: "system", content: SYSTEM_PROMPT },
        { role: "user", content: buildPrompt(apps, targetFamily) },
      ],
    }),
  });
  if (!res.ok) throw new Error(`Nosana inference endpoint returned ${res.status}`);
  const data = (await res.json()) as { choices?: { message?: { content?: string } }[] };
  const text = data.choices?.[0]?.message?.content ?? "";
  return parseSuggestions(text, apps);
}
