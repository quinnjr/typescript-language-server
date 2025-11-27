import * as fs from "node:fs";
import * as https from "node:https";
import * as path from "node:path";
import * as os from "node:os";
import { createGunzip } from "node:zlib";
import { pipeline } from "node:stream/promises";
import * as vscode from "vscode";

// GitHub repository info
const GITHUB_OWNER = "your-username";
const GITHUB_REPO = "typescript-language-server";
const SERVER_NAME = "typescript-language-server";

export interface DownloadProgress {
  downloaded: number;
  total: number;
}

/**
 * Get the platform-specific binary name
 */
export function getPlatformBinaryName(): string {
  const platform = os.platform();
  const arch = os.arch();

  let platformName: string;
  switch (platform) {
    case "linux":
      platformName = arch === "arm64" ? "linux-arm64" : "linux-x64";
      break;
    case "darwin":
      platformName = arch === "arm64" ? "darwin-arm64" : "darwin-x64";
      break;
    case "win32":
      platformName = "win32-x64";
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }

  return `${SERVER_NAME}-${platformName}`;
}

/**
 * Get the download URL for the server binary
 */
export function getDownloadUrl(version: string): string {
  const binaryName = getPlatformBinaryName();
  const ext = os.platform() === "win32" ? ".zip" : ".tar.gz";
  return `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases/download/v${version}/${binaryName}${ext}`;
}

/**
 * Get the path where the server should be stored
 */
export function getServerStoragePath(context: vscode.ExtensionContext): string {
  return path.join(context.globalStorageUri.fsPath, "server");
}

/**
 * Get the full path to the server executable
 */
export function getServerExecutablePath(context: vscode.ExtensionContext): string {
  const serverDir = getServerStoragePath(context);
  const ext = os.platform() === "win32" ? ".exe" : "";
  return path.join(serverDir, `${SERVER_NAME}${ext}`);
}

/**
 * Check if the server is already downloaded
 */
export function isServerDownloaded(context: vscode.ExtensionContext): boolean {
  const serverPath = getServerExecutablePath(context);
  return fs.existsSync(serverPath);
}

/**
 * Get the installed server version
 */
export function getInstalledVersion(context: vscode.ExtensionContext): string | undefined {
  const versionFile = path.join(getServerStoragePath(context), "version.txt");
  if (fs.existsSync(versionFile)) {
    return fs.readFileSync(versionFile, "utf-8").trim();
  }
  return undefined;
}

/**
 * Save the installed server version
 */
function saveInstalledVersion(context: vscode.ExtensionContext, version: string): void {
  const serverDir = getServerStoragePath(context);
  fs.writeFileSync(path.join(serverDir, "version.txt"), version);
}

/**
 * Fetch the latest release version from GitHub
 */
export async function getLatestVersion(): Promise<string> {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: "api.github.com",
      path: `/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/latest`,
      headers: {
        "User-Agent": "typescript-language-server-vscode",
        Accept: "application/vnd.github.v3+json",
      },
    };

    https.get(options, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        // Follow redirect
        const location = res.headers.location;
        if (location) {
          https.get(location, handleResponse).on("error", reject);
        } else {
          reject(new Error("Redirect without location"));
        }
        return;
      }

      handleResponse(res);

      function handleResponse(response: typeof res) {
        let data = "";
        response.on("data", (chunk) => (data += chunk));
        response.on("end", () => {
          try {
            if (response.statusCode !== 200) {
              reject(new Error(`GitHub API returned ${response.statusCode}: ${data}`));
              return;
            }
            const release = JSON.parse(data);
            const version = release.tag_name?.replace(/^v/, "") || release.name?.replace(/^v/, "");
            if (version) {
              resolve(version);
            } else {
              reject(new Error("Could not determine latest version"));
            }
          } catch (e) {
            reject(e);
          }
        });
      }
    }).on("error", reject);
  });
}

/**
 * Download a file from URL
 */
async function downloadFile(
  url: string,
  destPath: string,
  onProgress?: (progress: DownloadProgress) => void
): Promise<void> {
  return new Promise((resolve, reject) => {
    const request = (urlString: string) => {
      https.get(urlString, { headers: { "User-Agent": "typescript-language-server-vscode" } }, (res) => {
        // Handle redirects
        if (res.statusCode === 302 || res.statusCode === 301) {
          const location = res.headers.location;
          if (location) {
            request(location);
          } else {
            reject(new Error("Redirect without location"));
          }
          return;
        }

        if (res.statusCode !== 200) {
          reject(new Error(`Download failed with status ${res.statusCode}`));
          return;
        }

        const totalSize = parseInt(res.headers["content-length"] || "0", 10);
        let downloadedSize = 0;

        const file = fs.createWriteStream(destPath);

        res.on("data", (chunk) => {
          downloadedSize += chunk.length;
          if (onProgress && totalSize > 0) {
            onProgress({ downloaded: downloadedSize, total: totalSize });
          }
        });

        res.pipe(file);

        file.on("finish", () => {
          file.close();
          resolve();
        });

        file.on("error", (err) => {
          fs.unlink(destPath, () => {}); // Delete incomplete file
          reject(err);
        });
      }).on("error", reject);
    };

    request(url);
  });
}

/**
 * Extract a tar.gz file
 */
async function extractTarGz(archivePath: string, destDir: string): Promise<void> {
  // Use tar command for extraction (available on all Unix systems and modern Windows)
  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  await execAsync(`tar -xzf "${archivePath}" -C "${destDir}"`);
}

/**
 * Extract a zip file
 */
async function extractZip(archivePath: string, destDir: string): Promise<void> {
  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  if (os.platform() === "win32") {
    await execAsync(`powershell -command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`);
  } else {
    await execAsync(`unzip -o "${archivePath}" -d "${destDir}"`);
  }
}

/**
 * Download and install the server
 */
export async function downloadServer(
  context: vscode.ExtensionContext,
  version: string,
  outputChannel: vscode.OutputChannel
): Promise<string> {
  const serverDir = getServerStoragePath(context);
  const serverPath = getServerExecutablePath(context);

  // Ensure directory exists
  fs.mkdirSync(serverDir, { recursive: true });

  const downloadUrl = getDownloadUrl(version);
  const isWindows = os.platform() === "win32";
  const archiveExt = isWindows ? ".zip" : ".tar.gz";
  const archivePath = path.join(serverDir, `server${archiveExt}`);

  outputChannel.appendLine(`Downloading server from: ${downloadUrl}`);

  // Download with progress
  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Downloading TypeScript Language Server",
      cancellable: false,
    },
    async (progress) => {
      await downloadFile(downloadUrl, archivePath, ({ downloaded, total }) => {
        const percent = Math.round((downloaded / total) * 100);
        progress.report({ message: `${percent}%`, increment: 0 });
      });
    }
  );

  outputChannel.appendLine("Extracting server...");

  // Extract
  if (isWindows) {
    await extractZip(archivePath, serverDir);
  } else {
    await extractTarGz(archivePath, serverDir);
  }

  // Rename extracted binary to standard name
  const binaryName = getPlatformBinaryName();
  const extractedPath = path.join(serverDir, binaryName + (isWindows ? ".exe" : ""));
  
  if (fs.existsSync(extractedPath) && extractedPath !== serverPath) {
    fs.renameSync(extractedPath, serverPath);
  }

  // Make executable on Unix
  if (!isWindows) {
    fs.chmodSync(serverPath, 0o755);
  }

  // Clean up archive
  fs.unlinkSync(archivePath);

  // Save version
  saveInstalledVersion(context, version);

  outputChannel.appendLine(`Server installed: ${serverPath}`);

  return serverPath;
}

/**
 * Check for updates and prompt user
 */
export async function checkForUpdates(
  context: vscode.ExtensionContext,
  outputChannel: vscode.OutputChannel
): Promise<void> {
  try {
    const latestVersion = await getLatestVersion();
    const installedVersion = getInstalledVersion(context);

    if (installedVersion && installedVersion !== latestVersion) {
      const update = await vscode.window.showInformationMessage(
        `A new version of the TypeScript Language Server is available (${latestVersion}). Current: ${installedVersion}`,
        "Update",
        "Later"
      );

      if (update === "Update") {
        await downloadServer(context, latestVersion, outputChannel);
        vscode.window.showInformationMessage(
          "Server updated. Please restart the language server.",
          "Restart"
        ).then((action) => {
          if (action === "Restart") {
            vscode.commands.executeCommand("tsLspRust.restartServer");
          }
        });
      }
    }
  } catch (error) {
    outputChannel.appendLine(`Failed to check for updates: ${error}`);
  }
}

