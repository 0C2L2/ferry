import { Daytona } from "@daytona/sdk";

export interface VerifyResult {
  success: boolean;
  output: string;
}

/**
 * Dry-runs an AI-suggested install command inside a disposable Daytona
 * sandbox — never on the user's real machine. Only a command that exits
 * cleanly here gets shown to the user as "sandbox-verified".
 */
export async function verifyCommand(command: string): Promise<VerifyResult> {
  const apiKey = process.env.DAYTONA_API_KEY;
  if (!apiKey) throw new Error("DAYTONA_API_KEY is not set");

  const daytona = new Daytona({ apiKey });
  const sandbox = await daytona.create();
  try {
    // executeCommand runs an actual shell command; codeRun is for Python/JS/TS
    // code snippets in the sandbox's toolbox language, not arbitrary shell.
    const response = await sandbox.process.executeCommand(command);
    return { success: response.exitCode === 0, output: response.result };
  } catch (err) {
    return { success: false, output: err instanceof Error ? err.message : String(err) };
  } finally {
    await daytona.delete(sandbox);
  }
}
