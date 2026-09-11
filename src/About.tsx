import { useState, type MouseEvent } from 'react';
import { Code2, ExternalLink, Scale, Sparkles } from 'lucide-react';
import manifest from '../package.json';
import toolchain from '../toolchain.versions.json';
import license from '../LICENSE?raw';
import logo from '../src-tauri/icons/icon.png';
import { api, isDesktop } from './api';

export const appVersion = manifest.version;
const repository = 'https://github.com/yexca/quest-manager';
const dependencies = [
  ['Rust', toolchain.rust, 'Native backend, device operations and task queue'],
  ['Tauri', '2', `Desktop framework · JS API ${manifest.dependencies['@tauri-apps/api']}`],
  ['React', manifest.dependencies.react, 'Application interface'],
  ['TypeScript', manifest.devDependencies.typescript, 'Typed frontend development'],
  ['Node.js / npm', `${toolchain.node} / ${toolchain.npm}`, 'Development and build tools; not required to run the packaged app'],
  ['Vite', manifest.devDependencies.vite, 'Frontend development server and production build'],
  ['Android Platform-Tools', toolchain.adb.version, 'Official ADB client for headset communication'],
  ['Android AAPT2', toolchain.aapt2.version, 'APK metadata and resource processing'],
  ['Apktool', toolchain.apktool.version, 'Optional APK resource rebuilding'],
  ['Android Build Tools', toolchain.buildTools.version, 'APK signing and alignment with apksigner and zipalign'],
  ['Eclipse Temurin JRE', toolchain.jre.version, 'Bundled Java runtime for APK preparation'],
  ['Lucide', manifest.dependencies['lucide-react'], 'Interface icons'],
];

export function About() {
  const [error, setError] = useState<string | null>(null);
  const openRepository = (event: MouseEvent<HTMLAnchorElement>) => {
    if (!isDesktop) return;
    event.preventDefault();
    setError(null);
    void api.openProjectRepository().catch(cause => setError(String(cause)));
  };

  return <div className="about-page">
    <section className="about-intro card" aria-labelledby="about-project">
      <img src={logo} alt="Quest Manager logo" width="88" height="88" />
      <div><span className="about-version">Version {appVersion} · Windows</span><h2 id="about-project">A little order. More room to play.</h2><p>Quest Manager is a local desktop companion for Meta Quest. Manage applications and shared files, preview APKs, and prepare optional name, icon and compatibility changes through the official Android Debug Bridge.</p><p>Device operations stay on your computer and connected headset. No cloud account or analytics integration.</p></div>
    </section>

    <div className="about-info-grid">
      <section className="about-card card" aria-labelledby="about-credits">
        <h2 id="about-credits"><Sparkles size={19} />Development</h2>
        <p>This program was developed by <strong>yexca</strong> using <strong>Codex</strong>.</p>
        <dl className="about-facts"><div><dt>Development model</dt><dd>GPT-6-Astra</dd></div><div><dt>Project author</dt><dd>yexca</dd></div></dl>
      </section>
      <section className="about-card card" aria-labelledby="about-source">
        <h2 id="about-source"><Code2 size={19} />Source code</h2>
        <p>Explore the source, build instructions and project documentation on GitHub.</p>
        <a className="about-repository" href={repository} target="_blank" rel="noopener noreferrer" onClick={openRepository}>github.com/yexca/quest-manager<ExternalLink size={15} aria-hidden="true" /></a>
        <p className="about-caption">Opens in your browser.</p>
        {error && <p className="dialog-error" role="alert">{error}</p>}
      </section>
    </div>

    <section className="about-card card" aria-labelledby="about-dependencies">
      <h2 id="about-dependencies"><Code2 size={19} />Built with</h2>
      <p>Core frameworks and tools. Versions below come from this build's project manifests.</p>
      <table className="about-dependencies"><thead><tr><th scope="col">Component</th><th scope="col">Version</th><th scope="col">Role</th></tr></thead><tbody>{dependencies.map(([name, version, role]) => <tr key={name}><th scope="row">{name}</th><td>{version}</td><td>{role}</td></tr>)}</tbody></table>
      <p className="about-caption">Windows builds also use Microsoft C++ Build Tools and a Windows SDK. The desktop interface runs on Microsoft Edge WebView2. Third-party components retain their own licenses.</p>
    </section>

    <section className="about-card card" aria-labelledby="about-license">
      <h2 id="about-license"><Scale size={19} />AGPLv3 license</h2>
      <p>Copyright © 2026 yexca. Quest Manager is licensed under the GNU Affero General Public License, version 3 only (<strong>AGPL-3.0-only</strong>).</p>
      <p>You may use, modify and redistribute this program under the terms of this license. It is provided without any warranty.</p>
      <details className="about-license"><summary>Read the full license</summary><pre tabIndex={0} aria-label="GNU Affero General Public License version 3">{license}</pre></details>
    </section>
  </div>;
}
