# Notepad — portage Rust du Bloc-notes ReactOS

[![Télécharger notepad.exe](https://img.shields.io/badge/%E2%AC%87%EF%B8%8F_T%C3%A9l%C3%A9charger-notepad.exe_v0.1.0_(Windows_x64)-blue?style=for-the-badge)](https://github.com/mokhtarone/notepad-to-rust/raw/dist/notepad.exe)

**⬇️ [Télécharger notepad.exe (Windows 64 bits)](https://github.com/mokhtarone/notepad-to-rust/raw/dist/notepad.exe)** — binaire compilé et testé, 1,4 Mo. Somme de contrôle : [`SHA256SUMS.txt`](https://github.com/mokhtarone/notepad-to-rust/blob/dist/SHA256SUMS.txt). Note : Windows SmartScreen peut afficher un avertissement (binaire non signé) → *Informations complémentaires* → *Exécuter quand même*.

Portage en Rust du **Bloc-notes de ReactOS** (`base/applications/notepad`, fichiers `main.c` / `printing.c`) vers le crate [`windows` 0.62.2](https://crates.io/crates/windows/0.62.2). Le projet est développé sous Linux, **cross-compilé** vers `x86_64-pc-windows-gnu` et testé fonctionnellement sous **Wine**.

![Le portage Rust du Bloc-notes en action sous Wine](image/Capture%20d'écran_20261019_212835.png)

## Structure du dépôt

| Dossier | Contenu |
|---|---|
| `rust-version/` | Le portage Rust complet (binaire `notepad`) |
| `reference/` | Sources C de référence (ReactOS, GPL-2.0) |
| `original/` | Non versionné (binaires, archive bootcd) |

## Modules Rust

- `main.rs` — point d'entrée, boucle de messages, procédures de fenêtre (`main_wnd_proc`, `edit_wnd_proc`), moteur recherche/remplacement, ligne de commande (`/p`, chemin complet, fallback `.txt`)
- `dialogs.rs` — ouvrir/enregistrer, font, goto, wrap, barre d'état, curseur d'attente
- `printing.rs` — `PrintDlgW`, thread d'impression, rendu de page avec codes `&L/&C/&R/&D/&T/&F/&P`, mise en page
- `resources.rs` — menu et templates de dialogues construits en code (aucun `.rc`/`.res`)
- `settings.rs` — persistance registre (`RegQueryValueExW`/`RegSetValueExW`)
- `strings.rs`, `text.rs`, `globals.rs` — chaînes localisées, helpers, état global

## Compilation (cross-compile depuis Linux)

```bash
cd rust-version
rustup target add x86_64-pc-windows-gnu
cargo +stable build --target x86_64-pc-windows-gnu
# Binaire : target/x86_64-pc-windows-gnu/debug/notepad.exe
```

> Note : la compilation pour l'hôte Linux n'est pas supportée — le crate `windows` est un stub vide hors Windows et l'application est du Win32 pur.

## Test sous Wine

```bash
wine target/x86_64-pc-windows-gnu/debug/notepad.exe                # document vide
wine target/x86_64-pc-windows-gnu/debug/notepad.exe 'Z:\tmp\f.txt' # fichier existant
wine target/x86_64-pc-windows-gnu/debug/notepad.exe /p 'Z:\tmp\f.txt'  # impression
```

## Fonctionnalités validées sous Wine

- Ouverture/enregistrement de fichiers (roundtrip disque vérifié), détection de modification (`*` dans le titre)
- Accélérateurs : Ctrl+N/O/S/F/Z, F3, F5 (date/heure)
- Dialogues : Ouvrir, Enregistrer sous, Recherche (non modale), Police, Imprimer
- Prompt « enregistrer les modifications ? » à la fermeture d'un document modifié
- Barre d'état (position curseur, fins de ligne, encodage), wrap, drag & drop

## Limitation connue

Le dialogue « Mise en page » référence la ressource `DIALOG_PAGESETUP` via `MAKEINTRESOURCE` (fidèle au C, dont les ressources vivent dans le `.rc`) ; ce binaire n'embarque pas de `.res`, donc `PageSetupDlgW` échoue proprement. Le template mémoire existe dans `resources.rs` mais comdlg32 ne l'accepte pas directement (`PSD_ENABLEPAGESETUPTEMPLATE` passe par `FindResourceW`).

## Licence

**GPL-2.0-or-later** — œuvre dérivée de [ReactOS](https://reactos.org/) (sous GPL-2.0+). Voir [`LICENSE`](LICENSE) et le dossier `reference/`.
