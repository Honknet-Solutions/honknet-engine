use honknet_math::Vec2;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
pub type NodeId = u64;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Layout {
    Absolute,
    Flex { row: bool, gap: f32, wrap: bool },
    Grid { columns: u16, gap: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Style {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub grow: f32,
    pub padding: f32,
    pub margin: f32,
    pub opacity: f32,
    pub classes: Vec<String>,
    pub color: [f32; 4],
    pub background: [f32; 4],
    pub clip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Widget {
    Root,
    Panel,
    Label { text: String },
    Button { text: String, action: String },
    Image { resource: String },
    Scroll { offset: Vec2 },
    TextInput { value: String },
    RichText { markup: String },
    VirtualList { items: Vec<Value>, row_height: f32 },
    WorldSpace { entity: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub widget: Widget,
    pub style: Style,
    pub layout: Layout,
    pub children: Vec<NodeId>,
    pub bindings: HashMap<String, String>,
    pub rect: [f32; 4],
    pub visible: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    Click(NodeId),
    Change(NodeId, Value),
    Focus(NodeId),
    Blur(NodeId),
    Drag { node: NodeId, position: Vec2 },
    Drop { node: NodeId, target: NodeId },
}

#[derive(Default)]
pub struct UiTree {
    pub nodes: HashMap<NodeId, Node>,
    pub root: NodeId,
    pub focused: Option<NodeId>,
    pub hovered: Option<NodeId>,
    pub captured: Option<NodeId>,
    next: NodeId,
    dirty: HashSet<NodeId>,
    state: Value,
}

impl UiTree {
    pub fn create(&mut self, widget: Widget, layout: Layout, style: Style) -> NodeId {
        self.next += 1;
        let id = self.next;
        self.nodes.insert(
            id,
            Node {
                id,
                widget,
                style,
                layout,
                children: vec![],
                bindings: HashMap::new(),
                rect: [0.; 4],
                visible: true,
                enabled: true,
            },
        );
        self.dirty.insert(id);
        if self.root == 0 {
            self.root = id
        }
        id
    }
    pub fn append(&mut self, parent: NodeId, child: NodeId) {
        if let Some(p) = self.nodes.get_mut(&parent) {
            p.children.push(child);
            self.dirty.insert(parent);
        }
    }
    pub fn set_state(&mut self, state: Value) {
        self.state = state;
        for id in self.nodes.keys() {
            self.dirty.insert(*id);
        }
        self.apply_bindings()
    }
    fn apply_bindings(&mut self) {
        for n in self.nodes.values_mut() {
            for (prop, path) in n.bindings.clone() {
                if let Some(v) = lookup(&self.state, &path) {
                    match (&mut n.widget, prop.as_str(), v) {
                        (Widget::Label { text }, "text", Value::String(s)) => *text = s.clone(),
                        (Widget::Button { text, .. }, "text", Value::String(s)) => {
                            *text = s.clone()
                        }
                        (Widget::TextInput { value }, "value", Value::String(s)) => {
                            *value = s.clone()
                        }
                        (Widget::VirtualList { items, .. }, "items", Value::Array(a)) => {
                            *items = a.clone()
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    pub fn layout(&mut self, size: Vec2) {
        if self.root != 0 {
            self.layout_node(self.root, [0., 0., size.x, size.y])
        }
    }
    fn layout_node(&mut self, id: NodeId, rect: [f32; 4]) {
        let (children, layout, padding) = {
            let n = self.nodes.get_mut(&id).unwrap();
            n.rect = rect;
            (n.children.clone(), n.layout.clone(), n.style.padding)
        };
        match layout {
            Layout::Absolute => {
                for c in children {
                    let s = &self.nodes[&c].style;
                    self.layout_node(
                        c,
                        [
                            rect[0] + padding + s.margin,
                            rect[1] + padding + s.margin,
                            s.width.unwrap_or(rect[2] - 2. * padding),
                            s.height.unwrap_or(rect[3] - 2. * padding),
                        ],
                    )
                }
            }
            Layout::Flex { row, gap, .. } => {
                let total = if row { rect[2] } else { rect[3] };
                let fixed: f32 = children
                    .iter()
                    .map(|c| {
                        let s = &self.nodes[c].style;
                        if row {
                            s.width.unwrap_or(0.)
                        } else {
                            s.height.unwrap_or(0.)
                        }
                    })
                    .sum();
                let grow: f32 = children.iter().map(|c| self.nodes[c].style.grow).sum();
                let left = (total
                    - fixed
                    - gap * (children.len().saturating_sub(1) as f32)
                    - 2. * padding)
                    .max(0.);
                let mut cursor = if row {
                    rect[0] + padding
                } else {
                    rect[1] + padding
                };
                for &c in &children {
                    let s = &self.nodes[&c].style;
                    let primary = if row {
                        s.width.unwrap_or_else(|| {
                            if grow > 0. {
                                left * s.grow / grow
                            } else {
                                left / children.len().max(1) as f32
                            }
                        })
                    } else {
                        s.height.unwrap_or_else(|| {
                            if grow > 0. {
                                left * s.grow / grow
                            } else {
                                left / children.len().max(1) as f32
                            }
                        })
                    };
                    let r = if row {
                        [
                            cursor,
                            rect[1] + padding,
                            primary,
                            s.height.unwrap_or(rect[3] - 2. * padding),
                        ]
                    } else {
                        [
                            rect[0] + padding,
                            cursor,
                            s.width.unwrap_or(rect[2] - 2. * padding),
                            primary,
                        ]
                    };
                    self.layout_node(c, r);
                    cursor += primary + gap
                }
            }
            Layout::Grid { columns, gap } => {
                let cols = columns.max(1) as usize;
                let cw =
                    (rect[2] - 2. * padding - gap * (cols.saturating_sub(1) as f32)) / cols as f32;
                for (i, c) in children.into_iter().enumerate() {
                    let col = i % cols;
                    let row = i / cols;
                    let h = self.nodes[&c].style.height.unwrap_or(32.);
                    self.layout_node(
                        c,
                        [
                            rect[0] + padding + col as f32 * (cw + gap),
                            rect[1] + padding + row as f32 * (h + gap),
                            cw,
                            h,
                        ],
                    )
                }
            }
        }
    }
    pub fn hit_test(&self, p: Vec2) -> Option<NodeId> {
        self.nodes
            .values()
            .filter(|n| {
                n.visible
                    && p.x >= n.rect[0]
                    && p.x <= n.rect[0] + n.rect[2]
                    && p.y >= n.rect[1]
                    && p.y <= n.rect[1] + n.rect[3]
            })
            .max_by_key(|n| n.id)
            .map(|n| n.id)
    }
    pub fn diff(&mut self) -> Vec<Node> {
        let ids: Vec<_> = self.dirty.drain().collect();
        ids.into_iter()
            .filter_map(|i| self.nodes.get(&i).cloned())
            .collect()
    }
    pub fn visible_virtual_rows(
        &self,
        id: NodeId,
        viewport_height: f32,
    ) -> Option<std::ops::Range<usize>> {
        let n = self.nodes.get(&id)?;
        let Widget::VirtualList { items, row_height } = &n.widget else {
            return None;
        };
        let offset = 0.;
        let start = (offset / row_height).floor() as usize;
        let count = (viewport_height / row_height).ceil() as usize + 1;
        Some(start.min(items.len())..(start + count).min(items.len()))
    }
}

fn lookup<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = v;
    for p in path.trim_start_matches("$state.").split('.') {
        cur = cur.get(p)?
    }
    Some(cur)
}
