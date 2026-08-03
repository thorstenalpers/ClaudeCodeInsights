// Refuses to start a build while the app is running.
//
// `tauri-build` copies the sherpa DLLs next to the binary on every build, and a
// running app holds them open. Without this the build fails deep inside a build
// script with "The process cannot access the file because it is being used by
// another process", which says nothing about the window that is still open.
import { execFileSync } from 'node:child_process';

if (process.platform !== 'win32') process.exit(0);

const running = execFileSync('powershell', [
	'-NoProfile',
	'-Command',
	"(Get-Process claude-admin -ErrorAction SilentlyContinue | Measure-Object).Count"
])
	.toString()
	.trim();

if (running !== '0') {
	console.error('claude-admin is running. Close the app first: it holds the DLLs this build copies.');
	process.exit(1);
}
