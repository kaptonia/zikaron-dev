//! A grant code read one way wherever it is taken (typed on the check page, or in a file for the
//! vault or the check page), whatever white space came with it.

/// A code with a line break (CRLF) after it, a space before it, a space inside it, or a no-break space after
/// it reads as the clean code does, typed and in a file alike: the same answer, word for word. The code here
/// decodes to bytes that are not an entry, so the answer is the kit's refusal for that segment; white space
/// that were read as part of the code would be refused for another reason instead.
#[test]
fn a_code_with_white_space_reads_as_the_clean_code_typed_or_in_a_file() {
    let clean = format!("{}{}", zikaron_kit::tokens::BADGE_PREFIX, zikaron_kit::b64::encode(b"not an entry, a few bytes long"));
    let half = zikaron_kit::tokens::BADGE_PREFIX.len() + (clean.len() - zikaron_kit::tokens::BADGE_PREFIX.len()) / 2;
    let dir = std::env::temp_dir().join(format!("zk-test-code-space-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let said = |typed: &str| app::payloadx::take_full(typed).map(|t| t.hops).map_err(|f| format!("{} {}", f.said(), f.tail()));
    let in_file = |name: &str, text: &str| {
        let p = dir.join(name);
        std::fs::write(&p, text.as_bytes()).expect("file");
        said(&p.display().to_string())
    };
    let want = said(&clean);
    assert!(want.as_ref().err().is_some_and(|e| e.starts_with("PAYLOAD_REFUSED")), "the clean code is the kit's to judge: {want:?}");
    assert_eq!(in_file("clean", &clean), want, "the clean code in a file");
    for (form, text) in [
        ("a line break after it", format!("{clean}\r\n")),
        ("a space before it", format!(" {clean}")),
        ("a space inside it", format!("{} {}", &clean[..half], &clean[half..])),
        ("a no-break space after it", format!("{clean}\u{a0}")),
    ] {
        assert_eq!(said(&text), want, "typed, {form}");
        assert_eq!(in_file(form, &text), want, "in a file, {form}");
        assert_eq!(app::vaultx::take(&dir.join(form).display().to_string()).map_err(|f| format!("{} {}", f.said(), f.tail())), want, "the vault, {form}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// An identity backed up to a key file is sealed at no less than the backup floor (`keystore::BACKUP_FLOOR`),
/// in memory and in work: the file on disk is read back for its own parameters. The vault's floor in a shipped
/// build is the same constant.
#[test]
fn a_key_file_written_is_never_below_the_backup_floor() {
    if super::alone_in(module_path!(), "a_key_file_written_is_never_below_the_backup_floor") {
        return;
    }
    use app::action::{apply, Action, Applied};
    super::vault_open();
    let floor = app::keystore::BACKUP_FLOOR;
    assert_eq!(app::keystore::Params::standard(), floor, "the level written is the floor");
    let ctx = zikaron_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(zikaron_ui::skin::dress(&ctx));
    answers!(apply(&mut shell, Action::MakeAnchorKey), Applied::AnchorKey(_));
    let dir = std::env::temp_dir().join(format!("zk-r21b2-keyfile-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let pw = "a backup password";
    // The passcode opens the vault first (its own task), then the key file is written in the background.
    let answered = app::action::apply_settled(&mut shell, Action::BackupKey { pin: "27618394".into(), password: pw.into(), again: pw.into(), dir: dir.display().to_string() });
    let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while shell.tasks.in_flight(app::task::Kind::Keystore) && std::time::Instant::now() < until {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    shell.drain();
    let files: Vec<_> = std::fs::read_dir(&dir).expect("the folder").flatten().map(|e| e.path()).collect();
    assert_eq!(files.len(), 1, "one key file landed: {files:?}, answered {answered:?}, {:?}", shell.faults);
    let s = app::keystore::shape(&std::fs::read(&files[0]).expect("the key file")).expect("a key file's shape");
    let (mem, work) = (|n: usize, r: usize| n * r, |n: usize, r: usize, p: usize| n * r * p);
    assert!(mem(s.n, s.r) >= mem(floor.n, floor.r) && work(s.n, s.r, s.p) >= work(floor.n, floor.r, floor.p), "n={} r={} p={} below the floor", s.n, s.r, s.p);
    let _ = std::fs::remove_dir_all(&dir);
}

/// "A home laid incomplete is refused by name" is judged in one place: the refusal is made only in `home::lay`
/// (opening a home goes through it), never a second time on the way out.
#[test]
fn a_home_laid_incomplete_is_judged_in_one_place() {
    let makers: Vec<String> = super::shipped()
        .into_iter()
        .filter(|(name, _)| !name.ends_with("fault.rs"))
        .filter(|(_, text)| super::code_only(text).contains("Known::CannotLay"))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(makers, vec!["home.rs".to_string()], "one maker, the home layer (a folder module reads as one text)");
    let lay = super::code_only(&super::read_src_file("home.rs").expect("home.rs"));
    let at = lay.find("pub fn lay(").expect("lay");
    assert!(lay[at..].split("\n}\n").next().unwrap_or_default().contains("Known::CannotLay"), "inside lay");
}

/// `adopt` through the door: the request becomes the adoption page's own action, its anchors (the JSON the
/// command line read) made into the page's rows and judged by the page's one reading, each form by name; the
/// attestor and attestation go as given.
#[test]
fn an_adoption_through_the_door_is_the_adoption_pages_action() {
    let ctx = zikaron_ui::egui::Context::default();
    let shell = app::shell::Shell::boot(zikaron_ui::skin::dress(&ctx));
    let tx = format!("0x{}", "11".repeat(32));
    let content = format!("0x{}", "77".repeat(32));
    let turn = |anchors: Option<&str>| {
        let mut args = vec![("attestor".to_string(), "0xa".to_string()), ("attestation".to_string(), "0xs".to_string())];
        if let Some(a) = anchors {
            args.push(("anchors".to_string(), a.to_string()));
        }
        let q = zikaron_glue::door::Request { verb: "adopt".into(), home: "/h".into(), args };
        match app::door::turn_of(&shell, &q) {
            Ok(app::door::Turn::Act(app::action::Action::AdoptAnchors { rows, attestor, attestation })) => {
                assert_eq!((attestor.as_str(), attestation.as_str()), ("0xa", "0xs"), "as given");
                app::adoptx::rows_of(&rows).map(|r| r.len()).map_err(|f| f.which())
            }
            _ => panic!("adopt turns into the adoption page's action"),
        }
    };
    let good = format!(r#"{{"chainId":31337,"content":"{content}","payloadKind":"bare","tx":"{tx}"}}"#);
    assert_eq!(turn(Some(&format!("[{good},{good}]"))), Ok(2), "two rows, read as the page reads them");
    let shape = Err(Some(app::fault::Known::SettingsShape));
    assert_eq!(turn(Some(&format!(r#"[{good},{{"chainId":31337,"payloadKind":"bare","tx":"{tx}"}}]"#))), shape, "a member missing: refused by its line");
    assert_eq!(turn(Some(&format!(r#"[{{"chainId":"x","content":"{content}","payloadKind":"bare","tx":"{tx}"}}]"#))), shape, "a chain id not whole");
    assert_eq!(turn(Some(&good)), shape, "not an array: one line, refused");
    assert_eq!(turn(Some("[{}]")), shape, "an element with nothing in it");
    let none = Err(Some(app::fault::Known::FieldMissing));
    assert_eq!(turn(Some("[]")), none, "no anchors at all");
    assert_eq!(turn(None), none, "no --anchors");
    assert!(app::door::verbs().iter().any(|(v, fl)| *v == "adopt" && *fl == ["anchors", "attestor", "attestation"]), "the door's table lists it");
}

/// The app's node channel says its own place: two nodes said alike (one host, different paths) are two places
/// for every fact kept by place, whichever scheme reaches them.
#[test]
fn the_apps_node_channels_are_kept_apart_by_place() {
    for scheme in ["http", "https"] {
        let a = app::chainx::endpoint_at(&format!("{scheme}://node.example/eth/k1")).expect("a channel");
        let b = app::chainx::endpoint_at(&format!("{scheme}://node.example/polygon/k2")).expect("a channel");
        assert_eq!(a.name(), b.name(), "{scheme}: said alike");
        assert_ne!(a.place(), b.place(), "{scheme}: two places");
    }
}
