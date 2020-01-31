#![recursion_limit = "512"]

use stdweb::traits::IEvent;
use stdweb::web::Date;
use yew::services::ConsoleService;
use yew::{html, Component, ComponentLink, Html, Renderable, ShouldRender};

use egg::{EClass, EClassMatches, Id, ParsableLanguage, Pattern, RecExpr};
use egg_math::*;

struct Queried {
    pattern: Pattern<Math>,
    matches: Vec<EClassMatches>,
}

struct Model {
    console: ConsoleService,
    egraph: EGraph,
    query: Result<Queried, String>,
    added: Vec<Added>,
    examples: Vec<&'static str>,
    rewrite_groups: Vec<RewriteGroup>,
}

struct RewriteGroup {
    name: String,
    enabled: bool,
    rewrites: Vec<OptionalRewrite>,
}

fn rewrite_groups() -> Vec<RewriteGroup> {
    egg_math::rules()
        .iter()
        .map(|(&name, vec)| RewriteGroup {
            name: name.to_string(),
            enabled: true,
            rewrites: vec.iter().cloned().map(OptionalRewrite::new).collect(),
        })
        .collect()
}

impl RewriteGroup {
    fn render(&self, i: usize) -> Html<Model> {
        let applied: usize = self.rewrites.iter().map(|r| r.applied).sum();
        let matched: usize = self.rewrites.iter().map(|r| r.matched).sum();
        let percent = percent(applied, matched);
        let counts = format!("{}/{} ({:.0}%)", applied, matched, percent);
        html! {
            <div class="rewrite-group",>
                <input type="checkbox", checked=self.enabled, onclick=|_| Msg::ToggleRewriteGroup(i),></input>
                <details>
                    <summary> {counts} {" "} {&self.name} </summary>
                    { for self.rewrites.iter().enumerate().map(|(j, r)| r.render(i, j)) }
                </details>
            </div>
        }
    }
}

struct OptionalRewrite {
    applied: usize,
    matched: usize,
    enabled: bool,
    rewrite: Rewrite,
}

fn percent(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64 * 100.0
    }
}

impl OptionalRewrite {
    fn new(rewrite: Rewrite) -> Self {
        Self {
            applied: 0,
            matched: 0,
            enabled: true,
            rewrite,
        }
    }
}

impl OptionalRewrite {
    fn render(&self, gi: usize, i: usize) -> Html<Model> {
        let percent = percent(self.applied, self.matched);
        let counts = format!("{}/{} ({:.0}%)", self.applied, self.matched, percent);
        html! {
            <div class="rewrite",>
                <input type="checkbox", checked=self.enabled, onclick=|_| Msg::ToggleRewrite(gi, i),></input>
                <details>
                    <summary> {counts} {" "} {&self.rewrite.name}</summary>
                    <div class="lhs",> {self.rewrite.patterns[0].to_sexp()} </div>
                    <div class="rhs",> {format!("{:?}", self.rewrite.appliers[0])} </div>
                </details>
            </div>
        }
    }
}

struct Added {
    id: Id,
    expr: RecExpr<Math>,
}

impl Renderable<Model> for Added {
    fn render(&self) -> Html<Model> {
        html! { <tr> <td> {self.expr.to_sexp()} </td> <td> {self.id} </td> </tr> }
    }
}

enum Msg {
    AddExpr(String),
    AddQuery,
    RunRewrites,
    UpdateQuery(String),
    ToggleRewrite(usize, usize),
    ToggleRewriteGroup(usize),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        Model {
            console: ConsoleService::new(),
            egraph: EGraph::default(),
            query: Err("enter a pattern or expression".into()),
            added: vec![],
            examples: vec!["(+ 1 2)", "(* x (+ y z))", "(+ x (+ x (+ x x)))"],
            rewrite_groups: rewrite_groups(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UpdateQuery(s) => {
                self.query = Math::parse_pattern(&s)
                    .map(|pattern| {
                        let matches = pattern.search(&self.egraph);
                        Queried { pattern, matches }
                    })
                    .map_err(|e| e.to_string());
            }
            Msg::AddQuery => {
                if let Ok(pat) = &self.query {
                    match pat.pattern.to_expr() {
                        Ok(expr) => {
                            let id = self.egraph.add_expr(&expr);
                            self.update(Msg::UpdateQuery(expr.to_sexp().to_string()));
                            self.added.push(Added { expr, id });
                        }
                        Err(err) => {
                            self.query = Err(err.to_string());
                        }
                    }
                }
            }
            Msg::AddExpr(s) => {
                self.update(Msg::UpdateQuery(s));
                self.update(Msg::AddQuery);
            }
            Msg::ToggleRewrite(gi, i) => {
                self.rewrite_groups[gi].rewrites[i].enabled ^= true;
            }
            Msg::ToggleRewriteGroup(gi) => {
                self.rewrite_groups[gi].enabled ^= true;
            }
            Msg::RunRewrites => {
                let start_time = Date::now();
                let mut matches = Vec::new();

                for group in &mut self.rewrite_groups {
                    if !group.enabled {
                        continue;
                    }

                    for rule in &mut group.rewrites {
                        if rule.enabled {
                            let ms = rule.rewrite.search(&self.egraph);
                            rule.matched += ms.len();
                            matches.push((&rule.rewrite, &mut rule.applied, ms));
                        }
                    }
                }

                for (rule, applied, ms) in matches {
                    let actually_matched = rule.apply(&mut self.egraph, &ms);
                    *applied += actually_matched.len();
                }

                self.egraph.rebuild();

                let elapsed = Date::now() - start_time;
                self.console.log(&format!("Applied in {}s", elapsed));
            }
        };
        true
    }

    fn view(&self) -> Html<Self> {
        let query_message = match &self.query {
            Ok(q) => {
                let found: Vec<Id> = q.matches.iter().map(|m| m.eclass).collect();
                format!("Found at {:?}", found)
            }
            Err(s) => s.clone(),
        };

        html! {
        <div>
            <section id="add",>
                <form onsubmit=|e| {e.prevent_default(); Msg::AddQuery},>
                    <input oninput=|e| Msg::UpdateQuery(e.value),></input>
                </form>
                <p> {query_message} </p>
                <table>
                    <tr>
                        <th> {"expr"} </th>
                        <th> {"eclass"} </th>
                    </tr>
                    { for self.added.iter().map(Renderable::render) }
                </table>
            </section>
            <section id="examples",>
                <h3> {"Examples"} </h3>
                <div>
                    { for self.examples.iter().cloned().map(render_example) }
                </div>
            </section>
            <section id="stats",>
                <h3> {"Stats"} </h3>
                <div> { format!("Nodes: {}", self.egraph.total_size()) } </div>
            </section>
            <section id="eclasses",>
                <h3> {"EClasses"} </h3>
                <div> { for self.egraph.classes().map(render_eclass) } </div>
            </section>
            <section id="rewrites",>
                <h3> {"Rewrites"} </h3>
                <button onclick=|_| Msg::RunRewrites,>{"Run"}</button>
                <div>
                    { for self.rewrite_groups.iter().enumerate().map(|(i, g)| g.render(i)) }
                </div>
            </section>
        </div>
        }
    }
}

fn render_example(s: &'static str) -> Html<Model> {
    html! { <div onclick=|_| Msg::AddExpr(s.to_string()),> {s} </div> }
}

fn render_eclass(eclass: &EClass<Math, Meta>) -> Html<Model> {
    html! {
        <details class="eclass",>
            <summary> {eclass.id} </summary>
            <p>{format!("Size: {}", eclass.len())}</p>
            <p>{format!("Best: {}", eclass.metadata.best.to_sexp())}</p>
            <p>{format!("Cost: {}", eclass.metadata.cost)}</p>
        </details>
    }
}

fn main() {
    use stdweb::traits::IParentNode;
    use stdweb::web::document;
    yew::initialize();
    yew::App::<Model>::new().mount(document().query_selector("#app").unwrap().unwrap());
    yew::run_loop();
}
