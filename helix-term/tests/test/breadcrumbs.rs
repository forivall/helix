use std::fs;

use helix_term::{application::Application, config::Config};
use helix_view::{doc, editor::BreadcrumbPathOptions, view};

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn breadcrumb_tree_sitter_fallback_trail() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::with_suffix(".rs")?;
    fs::write(
        file.path(),
        "\
fn outer() {
    fn inner() {
        let x = 1;
    }
}
",
    )?;

    let mut config = Config::default();
    config.editor.breadcrumb.enable = true;
    config.editor.breadcrumb.path = BreadcrumbPathOptions::None;

    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .with_config(config)
        .build()?;

    let assertion = |app: &Application| {
        // Move the cursor into `inner` happened via the key sequence; the
        // trail must now contain it.
        let view_id = app.editor.tree.focus;
        let doc = doc!(app.editor);

        let breadcrumb = doc
            .breadcrumbs
            .get(&view_id)
            .expect("breadcrumb trail should exist for the focused view");

        assert!(!breadcrumb.is_empty());
        let names: Vec<_> = breadcrumb
            .iter()
            .map(|crumb| crumb.name.to_string())
            .collect();
        assert!(
            names.iter().any(|name| name == "inner"),
            "expected the trail to contain `inner`, got {names:?}"
        );
    };

    test_key_sequence(&mut app, Some("jj"), Some(&assertion), false).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn breadcrumb_disabled_by_default() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::with_suffix(".rs")?;
    fs::write(file.path(), "fn main() {}\n")?;

    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    let assertion = |app: &Application| {
        let doc = doc!(app.editor);
        assert!(
            doc.breadcrumbs.is_empty(),
            "no trails should be tracked while breadcrumbs are disabled"
        );
    };

    test_key_sequence(&mut app, Some("j"), Some(&assertion), false).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn breadcrumb_bar_hidden_when_empty_without_path() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::with_suffix(".rs")?;
    fs::write(
        file.path(),
        "\
fn outer() {
    fn inner() {
        let x = 1;
    }
}
// outside of all symbols
",
    )?;

    let mut config = Config::default();
    config.editor.breadcrumb.enable = true;
    config.editor.breadcrumb.path = BreadcrumbPathOptions::None;

    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .with_config(config)
        .build()?;

    // The cursor starts on line 1 (`fn outer() {`), so the bar shows its
    // trail...
    let assertion_visible = |app: &Application| {
        let view = view!(app.editor);
        let doc = doc!(app.editor);
        assert!(!view.breadcrumb_bar_empty(doc));
        assert_eq!(
            1,
            view.breadcrumb_offset(doc),
            "bar row stays reserved while the trail is non-empty"
        );
    };

    // ...but moving the cursor to the final comment line leaves it outside
    // every symbol. The bar then has no content, but its row must stay
    // reserved (no layout shift) so the covered text line can be revealed.
    let assertion_hidden = |app: &Application| {
        let view = view!(app.editor);
        let doc = doc!(app.editor);
        let trail = doc.breadcrumbs.get(&view.id);
        assert!(
            trail.is_none_or(|breadcrumb| breadcrumb.is_empty()),
            "trail should be empty outside of all symbols, got {trail:?}"
        );
        assert!(view.breadcrumb_bar_empty(doc));
        assert_eq!(
            1,
            view.breadcrumb_offset(doc),
            "bar row must stay reserved while the trail is empty to avoid layout shift"
        );
    };

    test_key_sequences(
        &mut app,
        vec![
            (None, Some(&assertion_visible)),
            (Some("ge"), Some(&assertion_hidden)),
        ],
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn breadcrumb_bar_not_reserved_for_document_without_symbols() -> anyhow::Result<()> {
    // A document that contains no symbols at all can never fill the bar
    // when `breadcrumb.path = "none"`, so its row is not reserved.
    let file = tempfile::NamedTempFile::with_suffix(".rs")?;
    fs::write(file.path(), "// just a comment, no symbols\n")?;

    let mut config = Config::default();
    config.editor.breadcrumb.enable = true;
    config.editor.breadcrumb.path = BreadcrumbPathOptions::None;

    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .with_config(config)
        .build()?;

    let assertion = |app: &Application| {
        let view = view!(app.editor);
        let doc = doc!(app.editor);
        assert!(
            view.breadcrumb_bar_empty(doc),
            "trail should be empty in a document with no symbols"
        );
        assert_eq!(
            0,
            view.breadcrumb_offset(doc),
            "no row should be reserved when the document has no symbols"
        );
    };

    test_key_sequence(&mut app, Some("j"), Some(&assertion), false).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn breadcrumb_bar_not_reserved_for_plain_text() -> anyhow::Result<()> {
    // A plain text file has no syntax to derive symbols from, so no row is
    // reserved either.
    let file = tempfile::NamedTempFile::with_suffix(".txt")?;
    fs::write(file.path(), "hello world\n")?;

    let mut config = Config::default();
    config.editor.breadcrumb.enable = true;
    config.editor.breadcrumb.path = BreadcrumbPathOptions::None;

    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .with_config(config)
        .build()?;

    let assertion = |app: &Application| {
        let view = view!(app.editor);
        let doc = doc!(app.editor);
        assert_eq!(
            0,
            view.breadcrumb_offset(doc),
            "no row should be reserved for documents without a syntax"
        );
    };

    test_key_sequence(&mut app, Some("j"), Some(&assertion), false).await?;

    Ok(())
}
