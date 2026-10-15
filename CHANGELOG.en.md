# Changelog

[中文](CHANGELOG.md) | English

## Next version (unreleased)

This version lets the command line write the data the desktop keeps sealed even while the desktop is closed or locked, authorised by you on the desktop, for a time you choose, and taken back whenever you like; the machine settings keep only the two the passcode gate needs in plain. **Data an earlier version left in an older form is not read and nothing converts it: bring it across by hand as in "Upgrading from an earlier version" in section 2 of the manual.**

### Command-line writing

- **Two switches**: the "Command line" group in Settings > Local data. "Command line writes through the desktop" decides whether the door is open (`--home`, no passcode); "Command line writes alone" asks for the passcode and a validity (days, hours and minutes, or never expiring) each time it is turned on. While writing alone is on, the desktop switch is greyed and the door is closed: there is one command-line path at any time.
- **Credential**: turning writing alone on writes a credential in the machine folder that only you can read and write; with it the command line writes the sealed ledger and anchor queue directly, taking no passcode and no `--key`. Turning the switch off removes it; its expiry removes it with a line saying so; a new master key makes it worthless, turns the switch off and says so; a changed passcode leaves it working; a credential failure counts no passcode try.
- **Taking turns with the lock**: while writing alone is on, the desktop takes the data folder's writer lock only for each write and lets it go after, reading things again first if the command line wrote; a desktop write that meets the command line writing is not done and says so, with a key to the switches. The command line meeting the desktop writing answers `E_HOME_HELD`; a data folder whose writer mark names another machine answers `E_HOME_OTHER_MACHINE`.
- **The folder after adoption**: once a plain ledger the command line made is taken in by the desktop ("Import key…", then "Adopt in place"), writing its original folder answers `E_ADOPTED` and the folder is never touched; the machine folder keeps only a blind list (the SHA-256 of a random salt followed by the root entry id), never a ledger name, path or entry id.
- **`retract`** writes a sealed ledger with the credential like the other entry-writing verbs; `init` is not taken there.
- New refusal reasons: `E_CREDENTIAL_MISSING`, `E_CREDENTIAL_EXPIRED`, `E_CREDENTIAL_SHAPE`, `E_HOME_HELD`, `E_HOME_OTHER_MACHINE`, `E_ADOPTED`.
- **Handing the ledger over is done on the desktop by hand**: `succeed --home` always answers `E_ON_DESKTOP` and is not handed to the desktop.

### Command-line reading, record kits and handoffs

- **The credential opens for reading too**: while the command line writes alone, `audit`, `check-grant`, `chain-check`, `depth`, `show --entry` and `kit-export` read the ledger in a desktop data folder with the same credential, refused for the same reasons as a write; `check-grant` so reads the issuer's whole ledger and can say whether a grant was revoked. `kit-export` from such a data folder takes the chosen records' originals and the "where it is anchored" line as the desktop does; `scan --ledger` takes its basis from that data folder's settings.
- **`kit-verify`**: the command line verifies a record kit record by record (is each original the one recorded, is it anchored), by the desktop's "Verify record" judge.
- **A record kit is one file**: the desktop and the command line export one `.zkit` file by default (the grant file's format), written and read item by item, with no 64 MiB limit any more; export a folder to publish at an address (the desktop's "Export as a folder", the command line's `--as folder`). Verifying takes a file or a folder.
- **The command line does not anchor again what the chain holds**: `anchor --form registry` asks the registry first whether this key already anchored those hashes, and answers `E_ON_CHAIN` and sends nothing when one is; used remotely through an agent, what this machine anchored is not paid for twice.
- **One place for originals**: a file the command line writes into a desktop data folder with `history --file` and the credential is recorded in the same index of signed files as the desktop's, so an exported record kit carries the original.
- **The idle desktop looks at the disk every second**: while writing alone is on and the desktop is unlocked, it wakes once a second to look at the disk, so what the command line writes shows without waiting for the mouse or the next timer.
- **Handing a ledger over**: `hand-over` writes the whole ledger into a handoff file and this place stops writing it; `take-over` takes a handoff as a follow-on only and this place writes again. A remote machine's command-line ledger and this machine's desktop can pass a ledger back and forth, one place writing at any time; this machine's side uses the credential, the desktop locked or not. New refusals `E_HANDED_OFF`, `E_HANDOFF`, `E_RECORDS`.

### Local data

- **Machine settings in two files**: `machine.json` holds only the language and the appearance (the passcode gate needs them); auto-lock, proxy, network, the command line's switches, the data folders used and the whole-machine backup record are in the encrypted machine settings, read once unlocked. Other settings an earlier version kept in `machine.json` are not read and go at the next save.
- **This version's form only**: local files in an earlier version's encrypted form, plain files and older key store forms are not read and nothing converts them (the reason is named and the file left as it is); upgrade by hand as in section 2 of the manual.

### Proxy

- **Following the system reads its configuration script**: when the system names an automatic configuration script (PAC), connections go as it says, then the proxies typed in. The system's automatic discovery ("automatically detect settings") is not read: `wpad` is never looked for by name, and a system with only that switched on counts as nothing set (straight), so no network the machine joins can hand it a script. The app fetches the script itself (never through a proxy; over `https` from anywhere, over `http` only from this machine, a literal loopback address or `localhost`; a `file:` script is not read) and runs it in an isolated JavaScript engine (QuickJS): only the standard PAC functions, no network, files, modules or timers, at most 2 seconds and 8 MiB a run; name lookups the script asks for are answered outside it, at most 1 second each. The ways the script names are tried in turn until one connects; `SOCKS` tries SOCKS5, then SOCKS4. A script that cannot be fetched, run or read leaves the way it would go without one (the proxies typed in, or straight); when that way does not connect either, the failure's next step says the script's trouble and the way out (nothing when it connects; the command line says it on its error line, with `--proxy` as the way out), the reason named: `PAC_UNFETCHED`, `PAC_SCHEME`, `PAC_REMOTE_HTTP`, `PAC_TOO_LARGE`, `PAC_FAILED`, `PAC_TIMEOUT`, `PAC_MEMORY`, `PAC_NO_FUNCTION`, `PAC_ANSWER`.
- **Reading each system**: Windows is read as the system answers the current settings (the script, the proxy list and its exceptions), the machine's when it sets one for every user; every way of writing the proxy list is read (no scheme, `http://`, one per protocol, `socks=` alone). Linux reads the desktop's stored settings first (KDE's `kioslaverc`; GNOME, Cinnamon and MATE's GSettings, from the dconf database or the keyfile, running no program), then the environment. macOS is read from the network configuration, the script included.
- **Proxies that ask who you are**: a proxy address may carry `user:password@` (HTTP Basic for an HTTP proxy, user name and password for SOCKS5); credentials in the environment, the system settings and a script's answer are used too. The password is kept in the encrypted machine settings, wiped once handed to the proxy, and never shown again: the settings page says "Password set". Saving the address again with the user name alone keeps the password.
- **HTTPS proxies and SOCKS4**: a custom proxy may be `https://` (TLS to the proxy and TLS to the node, each certificate checked) or `socks4://` (a user name only); host names are always left to the proxy to resolve.
- **A custom proxy leaves local networks alone**: private (`10/8`, `172.16/12`, `192.168/16`), link-local (`169.254/16`, `fe80::/10`), this machine and `.local` nodes always go straight.
- The command line's `--proxy` takes the same forms.
- **A proxy that fails is named**: when the leg to the proxy cannot be opened or is refused, the message says the proxy cannot be reached (`PROXY_UNREACHABLE`) and points at the proxy setting, not at the node.
- **A proxy that asks for credentials says so**: a proxy asking for a user name and password when none were given (HTTP 407, SOCKS5 asking for them, SOCKS4 not taking the user ID) is "The proxy asks for a user name and password" (`PROXY_CREDENTIALS_MISSING`); one refusing those given is "The proxy refused the user name and password" (`PROXY_CREDENTIALS_REFUSED`); the next step points at the custom proxy's user name and password, no longer telling someone already following the system to go back to it. The command line answers `E_UNREACHABLE` with the same `token`, and says how `--proxy` carries credentials.
- **The custom proxy in cells**: type, host, port, user name and password each in a cell, typed as they are (no hand-written `%40` or `%3A`); the port takes 1 to 65535, a wrong cell is framed red with its reason and "Save" waits; SOCKS4 has a user ID cell only; a whole proxy address pasted into the host cell is split into the cells. The password is kept encrypted as before and never shown once saved; with the cell left empty it stays for the same type, host, port and user name, and does not go along when any of them changes. The command line's `--proxy` is written as before.

### Other

- The passcode gate no longer switches the language (the first-run wizard still does).
- Waiting for a receipt, a node a few blocks behind the others no longer ends the wait early; it runs to its end. When a send through the desktop answers "not in a block yet", `count` is the seconds actually waited.
- A new message waits for the last one to fade out instead of lying over it; full-window pages (the passcode gate, recovery, first run) keep their content clear of the scroll bar; the validity's three cells centre their numbers.
- **The open desktop follows what the command line writes**: once a second the desktop looks at the ledger on disk, and entries the command line wrote alone show at once in the ledger, the pending count, the home cards and the local data page's summary; with "auto anchor" on, the send card opens for them as after a record of the desktop's own.
- **The pending queue follows the ledger and the chain**: entries a self-audit finds on chain that still wait in the queue unsent (the command line anchored them itself, say) leave it, recorded as anchored, and are never sent again; entries the chain does not hold that are not queued (an adopted ledger's, say) join it after the next self-audit.
- The message after a record names the sequence number actually written (no longer one short when the command line wrote meanwhile).
- While writing alone is on, a command with `--home` answers `E_DESKTOP` with `detail` `ALONE` (write without `--home`), no longer that the desktop is locked or not open.
- **A configuration script comes from this machine or over https**: a system's script address that is `http` on another machine is neither fetched nor run, nor is one a redirect points to on the way; the way goes without it (`PAC_REMOTE_HTTP`), and when that way does not connect either the failure's next step says "Proxy script is on remote http; not used. Set a custom proxy." Nobody on the network can hand in a script. **A remote `http` configuration script used before upgrading (a company's or a local network's) is no longer used after: enter the proxy as a custom proxy.**
- **The command line reads the terminal's proxy variables**: with `--proxy system` (the default), `https_proxy`, `http_proxy`, `all_proxy` and `no_proxy` (either case) are followed when any proxy among them is set, the same on macOS, Windows and Linux, and only when none is set are the system's proxy settings read; a proxy set in the terminal and not in the system is now followed by the command line. The desktop is unchanged.
- A release carries no path of the machine that built it: the proxy script engine's C sources (QuickJS) are built without assertions, as its own release builds are, so on Linux they no longer embed the build directory's full path.

## 0.1.2

This release reworks how the app talks to nodes (proxies, rate limits, comparing several nodes), lets the command line write and put records on chain through the running desktop app, adds resending a stuck batch with higher fees, and moves local data to a new version of its encrypted format. The ledger law and the record kit format (zikaron/1) are unchanged: ledgers, record kits and data folders written by 0.1.1 read and write as before in 0.1.2. Encrypted local files written by 0.1.2 cannot be read by 0.1.1, so do not open the same data with 0.1.1 after upgrading.

### Network and nodes

- **Proxy**: Settings > Network has a new "Proxy" setting: System (the default), Off, or Custom proxy (`http://` or `socks5://`). Nodes on this machine are never reached through a proxy. The manual's "Network" section covers VPNs.
- **Steadier connections**: when a node's address resolves to several IP addresses, each is tried; every request has one overall deadline; a chain read cut off mid-connection is asked once more (a transaction is never sent twice); a rate-limited node is asked again after waits from one table; one request goes to every node at the same time.
- **Comparing nodes by what each request needs**: each kind of request compares only the facts its result depends on; the chain head is the lowest of the nodes', and a node too far behind is named; nodes serving another chain are left out.
- **Keys in node addresses stay private**: an access key inside a node address never appears in any message, detail or log, whatever the address's form (IPv6, no port given, the key in the path).
- **The main network's registry is checked when saved**: a registry you enter by hand whose code is not the build ZIKARON pins is not saved.
- IPv6 node addresses can be written in brackets (for example `http://[::1]:8545`).
- Every request's `User-Agent` names the product only.

### Putting records on chain

- **Resend with higher fees**: when a batch sits in the nodes' pools without being included, "Resend with higher fees" signs it again at the same nonce at the price now; the old and the new transaction are both watched and the first included counts. A batch is resent at most three times.
- **When no node holds a batch**: if its nonce is still unused, "Send again at the price now" sends it again; if another transaction used its nonce, the batch lapses and its entries go back to the queue. 0.1.1 kept waiting for a receipt forever.
- **The tip is read correctly**: 0.1.1 always read the tip as its 1 gwei cap; the tip the chain actually paid is read now.
- **The balance check before sending** asks every node at the lowest block they have all reached and compares them; a node that gives no answer is never read as zero.
- **The confirmation card says when fees fall back**: when the base fee cannot be read and the fallback is used, the card says so.
- **The command line tells two refusals apart**: a node that refuses the gas estimate because the transaction would fail (`E_GAS_REFUSED`), and one that refuses for its own reasons such as a rate limit (`E_UNREACHABLE`).

### Command line

- **Writing and putting on chain through the desktop**: with `--home <data folder>`, eleven writing verbs (`adopt` among them) are done by the running desktop app, the entry shows in its window at once, and the command line holds no key. `anchor` through the desktop follows "Putting on chain from the command line" in Settings > Network: send automatically, or queue only and wait for you to send from the desktop.
- **Enable command line**: a new switch in Settings > Local data makes `zikaron` available in any terminal (a link in `/usr/local/bin` on macOS and in `~/.local/bin` on Linux, this user's `PATH` on Windows).
- **Adopting anchor proofs judges a co-signature whole**: a co-signature with only one of its two halves is refused, on the page and on the command line alike.
- **A rerun never pays twice**: without `--home`, `anchor` records the hashes it signed; when the same command runs again it asks by hash first, answers an included transaction as included and signs no new nonce, and answers `E_TX_VOID` when what it sent lapsed.
- **`zikaron contract`** prints the command line's own contract (verbs, flags, exit codes, refusal reasons), made from the same tables in the code.
- A usage error no longer echoes the value you typed (for example a mistyped private key); it names only the flag and where it went wrong.
- Messages on standard error follow the system language, Chinese or English.
- New `--key-file` reads the private key from a file; key files derived with pbkdf2 are accepted too.
- `anchor --form registry` without `--registry` is a usage error (exit 2) before any node is asked.

### Local data

- **A new version of the encrypted format**: the whole file header is authenticated, and file names and the part of each header that says which file it is are computed with this machine's key, so someone holding the disk cannot tell from entry or grant numbers what this machine keeps. Older files are read, only the new version is written, and nothing is migrated in bulk.
- **Files that cannot be read are never overwritten**: a file written by a newer version or damaged is left as it is and named with one of six reasons; settings a newer version wrote are carried back as they are.
- **One writer per data folder across machines**: when a data folder on a synced or removable drive is open on two machines, the later one is read-only and can take over with "Write from this machine"; a writer mark that cannot be read leaves the data read-only and the mark untouched.
- **A data folder from elsewhere is refused before it opens**: one that belongs to another identity or another machine is refused without writing a byte.
- **Entries land on FAT and exFAT volumes** (0.1.1 failed on every entry there).
- **Whole-machine backups** carry the read-only network list; the backup lamp has four colours (up to date, entries added since, never backed up, last backup failed), and a failed backup is named under Alerts.
- **A backup never replaces a file**: when the name chosen for a backup is already taken, the next free number is used.
- **Saving on Windows waits out a busy file**: when another program (a virus scanner, for example) holds a file for a moment, the save tries again for up to a second instead of failing with "no permission".
- **Programs the app starts never hold its lock**: on macOS and Linux, a program started from the app no longer keeps the data folder locked after the app quits.
- A broken key file, or one with parameters out of bounds, is refused before any key derivation runs, so it cannot tie up the machine.
- Recovery words that were shown are wiped from memory on hide, lock and quit; a passcode is wiped as soon as it has been handed on.

### Interface

- File dialogs no longer freeze the window.
- Switching identity uses the identity menu again, with its title and the blue frame on the identity in use.
- The first layer of the English interface no longer carries Chinese node messages; what each node said is in folds such as "What each said".
- English in toasts and key-value tables wraps between words.
- The "Third-party licences" list scrolls smoothly and fills its frame.
- Long type labels are shortened with the full text on hover; the English for "存证" is now Record.
- Wording across the settings pages is shorter.

### Fixes

- A batch that no node held stayed waiting for its receipt forever.
- How far a backup is behind counts the entries added since the backup; 0.1.1 could show an old backup as current after a data folder was deleted and recording went on.
- Importing an identity that is already here answers by name (already on this machine, or the name is taken).
- A chain height a node returns in the wrong shape is refused by name, never read as 0.
- A record place given on more than one line is refused by name instead of reading only the first line.
- A grant code's shape is checked first, so a half-pasted code no longer says the root cannot be reached.
- A strict ledger read names a zero-byte entry file instead of skipping it.
- A grant file's length header is read in one spelling only.
- Closing a panel or overlay no longer flashes its text in the placeholder colour while it fades.
- Quitting no longer waits for network reads to finish: on every system an exchange in flight is cut within a quarter of a second.
- After the app quits, the command line with `--home` is told at once that the app is not running; on Windows it could be left waiting on a channel the app had already closed.
- A grant code pasted from a messenger or an email with line breaks, spaces or invisible marks (zero-width characters, a byte-order mark) is read as the code itself.

### Known limitations

- The Windows and Linux builds are built and tested automatically but have not yet been tried widely on real machines; please report problems on GitHub.
- When a data folder's writer mark cannot be read, the pop-up messages use general wording; the bar at the top of the page says what actually happened.

## 0.1.1

This version adds Windows and Linux clients; the preset networks grow to four and each identity uses its own network; the gas fee is worked out from the chain; reading other people's material can read several chains at once; slow operations in the window run in the background. The ledger law and the record kit format (zikaron/1) are unchanged: ledgers, record kits and data folders written by 0.1.0 are read and written by 0.1.1 as before.

### Platforms

- **Windows**: Windows 10 and 11 on x86_64, as a zip: unpack and run; the programs carry their own runtime. Machine data lives in `%LOCALAPPDATA%\ZIKARON\` by default. The programs are not signed; see section 2 of the manual for installing and the first start.
- **Linux**: `.deb` and AppImage for x86_64, needing glibc 2.31 or later (Ubuntu 20.04, Debian 11 and later).
- Every package carries the third-party licences; they are also under "About" in the app.

### Networks

- **Four preset networks**: Ethereum mainnet, Sepolia testnet, Arbitrum One, OP Mainnet. The registry contract has the same address and the same build on the three production networks.
- **Each identity uses its own network**: pick the network when you create or import an identity; identities on one machine may be on different chains. Nodes, chain id and contract filled in by hand belong to the identity that filled them in and do not change when the preset table does. A 0.1.0 identity is given this machine's former network once, on upgrade.
- **New mainnet preset nodes**: `rpc.flashbots.net`, preset in 0.1.0, keeps only the last ten to twenty thousand blocks of logs, so syncing reported "Nodes returned different results.". When a data folder opens with the 0.1.0 pair of nodes unchanged, they are replaced with the new pair; edited ones are left alone.
- **Read-only networks**: Settings > Network can add any number of read-only networks, used only to check others' material and never to send transactions. Before one is read, the code fingerprint of its registry contract is checked; on a mismatch it is not read.
- **Reading others' material reads several chains**: Verify record, Others' ledger, Due diligence and Verify grant read the main network and every read-only network, and say chain by chain whether it was read. A chain that was not read is marked "Chain not read", never taken as absent.
- **Verifying offers to add a network**: when a record kit names a network you have not added, the kit is still checked but its anchors are not; the page names the chain, and "Add" takes the chain id, registry contract and start block to Settings.

### Anchoring and fees

- **The gas fee comes from the chain**: the cap is (base fee × 2 + tip) × gas limit. The tip is the median of the last 20 blocks' paid tips, at most 1 gwei. 0.1.0 used fixed values.
- **The gas limit follows the estimate**: the node's estimate × 1.5 (rounded up), at most 200,000; an estimate above 200,000 is refused and nothing is sent. 0.1.0 always carried 200,000, so its balance threshold and the cap on the confirm card were about ten times too high. The balance check before sending, the cap on the confirm card and the transaction sent use the same number.
- **The estimate is made afresh every time**: every time the confirm card opens; changing the node, the network or the data folder voids it. 0.1.0 reused the last estimate and, after switching chains, could send with another chain's figure.
- **Amounts in significant digits**: small amounts no longer show as `0.0000 ETH`; caps round up, balances round down.
- **The receipt is awaited until the deadline**: after sending, the receipt is asked for until confirmed or timed out; a node briefly unreachable is not a failure.
- **The command line estimates before anchoring too**: if the node says the transaction would fail, or the estimate is above 200,000, nothing is sent and the refusal is `E_GAS_REFUSED` (exit 1). The 0.1.0 command line did not estimate and broadcast transactions that would fail.

### Syncing and reading the chain

- **Only the logs that matter are asked for**: queries carry the senders this machine cares about instead of pulling back everyone's records from the registry contract, so syncing no longer slows down as the chain's record count grows.
- **A refused query is asked again**: when a node refuses a query it is asked again as it was; only after three refusals is the range split by the limit the node gave. A limit written with thousands separators is understood.
- **What has been checked is not asked again**: records several nodes agreed on are kept on this machine and not asked for again; new records are asked for; a replaced block is asked again. Settings > Local data > Advanced options can recheck everything.
- **Nodes are compared only on the fields a verdict uses**: differences in unrelated fields are no longer a disagreement. With the two preset nodes, 0.1.0's self-check sometimes reported a disagreement, and importing on-chain records could be refused because of it.

### The window

- **Slow work runs in the background**: estimating gas (the primary key says "Estimating"), sending ("Sending"), fingerprinting a file, recording with a file ("Recording"), changing the data folder ("Moving"; meanwhile that data can be read but not written). The window no longer freezes.
- **Pickers with search**: "Choose a record" and "Recent addresses" in New grant, the address book, the target entry of a note, "Choose entries" when exporting a record kit, and switching identity open as floating cards with a search field.
- **Filter by date**: Records, Ledger, Grants and My grants can be filtered by on-chain date, picked from a calendar.
- **Verify pages**: both the Recorder and the User have Verify grant, Verify record and Others' ledger.
- **Grant cards** show when the grant went on chain.
- **The network page** shows only the network's name on its top line.
- **Chinese text on Windows** follows the system and uses Microsoft YaHei UI, with a real bold for titles; when it is not there, the embedded Noto Sans SC is used.
- **Hide entries deleted on this machine**: a new switch in Settings > Local data; when on, the Records and Ledger lists leave out entries deleted before they went on chain, which stay on this machine only; entries deleted after going on chain are still listed. Display only, off by default.

### Record kits and verification

- **A record kit says where it is anchored**: on export, a line `anchored-on: eip155:<chain id> · registry <contract> · from <start block>` is added at the end of the note.
- **Verification results are written down**: each record kit verified leaves a result file under `kits/verified/` (format `zikaron.kit-verification/1`) naming only the chains actually read, for other apps to read.

### Command line

- `--endpoint` accepts `https://` and checks the certificate.
- Writes are audited offline first: a write that would break the ledger is refused with `E_WOULD_BREAK`, and not a byte is written.
- `history` gains `--file`, recording a file by the same convention as the app; when the content does not qualify, standard error gets one more line saying why.
- `check-grant` reads `.zkgrant` grant files exported by the app; `--ledger` can point at a ledger mirror folder or a record kit.
- `audit --out` saves a copy of this audit's input.
- Exit codes, refusal reasons and what standard error says are set out in the repository's `CLI-SCHEMA.md`.

### Fixes

- Imported identities (recovery phrase, private key, key file) could not create a ledger. Now a ledger checked against the chain can be written, wherever it came from.
- Importing a private key as the primary identity at first start now requires exporting a key file at the same time, so it can be recovered later.
- A damaged key store no longer sends you into the first-start wizard; it says "The key store cannot be read", with the reason and the file's path.
- After a key change on the command line the old key can no longer write; with two roots in a ledger and no `--root`, writes are refused.
- "Change data folder…" accepts only an existing data folder or an empty folder, and no place inside the current one (0.1.0 copied the folder into itself, layer by layer).
- "Copy grant code" on a sublicense carries the whole chain up to the root; the issuer's note is kept on the last hop only.
- "Import grants folder…" checks the files one by one; a bad one does not hold up the rest.
- Exporting a record kit with an entry that is not in the ledger is an error instead of being skipped quietly.
- A passcode longer than the field can take is refused instead of cut short.
- Restoring a whole-machine backup checks every member's file name and refuses any that climbs out of its folder.

### Known limitations

- When anchoring from the command line, any node refusal of the estimate is reported as the transaction would fail, without telling rate limits and the like apart; nothing is sent, and the node's own words are in `detail`.
- When the base fee cannot be read and the fallback fee is used, the confirm card does not say it is the fallback.
- With only one node configured, logs and receipts are not cross-checked against each other; two or more nodes are not affected.
- The official OP Mainnet node sometimes answers that the range is limited; asking again usually gets through.
- The Windows and Linux clients have passed automated builds and tests but have not yet been tried widely on real machines; please report problems on GitHub.

## 0.1.0

First release: macOS (Apple silicon).
