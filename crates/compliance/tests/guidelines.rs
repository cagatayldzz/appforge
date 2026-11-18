use appforge_compliance::Guidelines;

fn page(detail: &str, extra: &str) -> String {
  let other_rules = (10..70).map(|n| format!("<li id='4.{n}'><strong>4.{n}</strong> Example</li>")).collect::<String>();
  format!(
    "<main><ul><li id='1.2'><strong>1.2 Community</strong></li><li id='2.1'><strong>2.1 Completion</strong></li>
    <li id='purchases'><span id='3.1.1'></span><strong>3.1.1 Purchases:</strong><ul>
    <li id='other-payment'><span id='3.1.1a'></span><strong>3.1.1(a) External payment:</strong> {detail}</li></ul></li>
    <li id='4.2'><strong>4.2 Design</strong></li><li id='5.1.1'><strong>5.1.1 Privacy</strong></li>
    {other_rules}{extra}</ul></main>"
  )
}

#[test]
fn guidelines_keep_nested_ids_source_links_and_last_change() {
  let base = Guidelines::from_html(&page("A &amp; B", "<li id='5.7'>Removed</li>"), &Guidelines::default()).unwrap();
  let rule = base.rule("3.1.1a").unwrap();
  assert_eq!(rule.number, "3.1.1(a)");
  assert_eq!(rule.topic, "Purchases");
  assert!(rule.url().ends_with("#other-payment"));
  assert!(base.context(["3.1.1(a)"], "", 1000).contains("A & B"));
  let changed_html = page("C", "<li id='5.8'>New</li>");
  let changed = Guidelines::from_html(&changed_html, &base).unwrap();
  assert_eq!(changed.changes.added, ["5.8"]);
  assert_eq!(changed.changes.modified, ["3.1.1(a)"]);
  assert_eq!(changed.changes.removed, ["5.7"]);
  assert_eq!(Guidelines::from_html(&changed_html, &changed).unwrap().changes.modified, ["3.1.1(a)"]);
  assert!(Guidelines::from_html("<main>Maintenance</main>", &changed).is_err());
}
