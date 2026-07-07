# Backup & Restore Plan: GPG Keys & `pass`

Since the `pass` command inherently encrypts all of your passwords using your GPG public key, the actual `~/.password-store` directory is safe to store in the cloud, provided your **GPG Private Key** is kept strictly isolated and secure.

This plan outlines a highly secure, software-only strategy without requiring a hardware YubiKey.

## Phase 1: Syncing the Password Store (`pass`)

Because all files inside `~/.password-store` are encrypted (`.gpg` files), it is an industry-standard practice to sync this directory using a private Git repository.

1. **Initialize Git inside pass:**

   ```bash
   pass git init
   ```

2. **Add a Private Remote:** Create a _private_ repository on GitHub (e.g., `github.com/kuranne/password-store`) and link it:

   ```bash
   pass git remote add origin git@github.com:kuranne/password-store.git
   ```

3. **Push to Sync:**

   ```bash

   pass git push -u --all
   ```

   _Note: From now on, whenever you add/edit a password, `pass` automatically creates a git commit. You just need to run `pass git push` to back it up._

## Phase 2: Backing up the GPG Private Key

The only thing that can decrypt your passwords is your GPG private key. This **MUST NEVER** be pushed to GitHub.

1. **Find your Key ID:**

   ```bash
   gpg --list-secret-keys --keyid-format=long
   ```

2. **Export the Private Key securely:**

   ```bash
   gpg --export-secret-keys --armor <YOUR_KEY_ID> > my-private-key.asc
   ```

## Phase 3: Securing the Private Key

You have two great options for storing `my-private-key.asc`:

- **Option A (The Cloud Vault):** Create a "Secure Note" in a trusted Zero-Knowledge password manager (Bitwarden, 1Password, Proton Pass) and paste the entire contents of `my-private-key.asc` into it.
- **Option B (The USB Vault):** Move `my-private-key.asc` to a physically encrypted USB flash drive and put it in a safe place.

_(Important: Delete `my-private-key.asc` from your computer immediately after storing it!)_

## Phase 4: Restoration on a New Machine (via `bkzyn`)

When you run `install.sh` on a brand-new macOS or Linux machine, here is the manual workflow you will follow to restore your passwords:

1. **Import your GPG Key:**
   - **If using Option A:** Install your Password Manager CLI (e.g., `brew install bitwarden-cli`), log in, fetch the note, and pipe it to GPG:

     ```bash
     bw get notes "GPG Private Key" | gpg --import
     ```

   - **If using Option B:** Plug in your USB drive and run:

     ```bash
     gpg --import /Volumes/USB/my-private-key.asc
     ```

2. **Trust the Key:**

   ```bash
   gpg --edit-key <YOUR_KEY_ID>
   # Type: trust -> 5 (Ultimate) -> save
   ```

3. **Clone the Password Store:**

   ```bash
   if [ -z $XDG_DATA_HOME ]; then
      git clone git@github.com:kuranne/password-store.git $XDG_DATA_HOME/.password-store
   else
      git clone git@github.com:kuranne/password-store.git ~/.password-store
   fi
   ```

You now have full access to your `pass` passwords on the new machine, completely securely!
