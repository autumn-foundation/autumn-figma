import json, sys
out = sys.argv[1]

def bb(x, y, w, h): return {"x": x, "y": y, "width": w, "height": h}
def solid(r, g, b, a=1, var=None, opacity=None):
    p = {"type": "SOLID", "color": {"r": r, "g": g, "b": b, "a": a}, "blendMode": "NORMAL"}
    if opacity is not None: p["opacity"] = opacity
    if var: p["boundVariables"] = {"color": {"type": "VARIABLE_ALIAS", "id": var}}
    return p
def alias(v): return {"type": "VARIABLE_ALIAS", "id": v}
WHITE = (1, 1, 1); GRAY = (0.42, 0.447, 0.502); BORDER = (0.898, 0.906, 0.922); PRIMARY = (0.486, 0.227, 0.929)

def vector(id_, x, y):
    return {"id": id_, "name": "Vector", "type": "VECTOR", "absoluteBoundingBox": bb(x, y, 12, 9),
            "fills": [], "strokes": [solid(*WHITE)], "strokeWeight": 2}

def icon_instance(id_, x, y, refs=None, visible=True):
    n = {"id": id_, "name": "Icon", "type": "INSTANCE", "componentId": "1:30", "visible": visible,
         "absoluteBoundingBox": bb(x, y, 16, 16), "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "FIXED",
         "children": [vector(id_.replace("I", "I") + ";1:31" if id_.startswith("I") else "I" + id_ + ";1:31", x + 2, y + 4)]}
    if refs: n["componentPropertyReferences"] = refs
    return n

def text(id_, name, chars, x, y, w, h, size=16, weight=400, lh=24, fill=None, style_id=None, refs=None, extra=None):
    n = {"id": id_, "name": name, "type": "TEXT", "characters": chars, "absoluteBoundingBox": bb(x, y, w, h),
         "style": {"fontFamily": "Inter", "fontPostScriptName": None, "fontWeight": weight, "fontSize": size,
                   "lineHeightPx": lh, "lineHeightUnit": "PIXELS", "letterSpacing": 0,
                   "textAlignHorizontal": "LEFT", "textAlignVertical": "TOP"},
         "textAutoResize": "WIDTH_AND_HEIGHT", "layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG",
         "fills": [fill or solid(0.067, 0.094, 0.153)]}
    if style_id: n["styles"] = {"text": style_id}
    if refs: n["componentPropertyReferences"] = refs
    if extra: n.update(extra)
    return n

def button_variant(id_, name, x, pad_v, pad_h, radius, size, icon_id, label_id):
    return {"id": id_, "name": name, "type": "COMPONENT", "absoluteBoundingBox": bb(x, 0, 120, 48),
            "layoutMode": "HORIZONTAL", "primaryAxisAlignItems": "CENTER", "counterAxisAlignItems": "CENTER",
            "itemSpacing": 8, "paddingLeft": pad_h, "paddingRight": pad_h, "paddingTop": pad_v, "paddingBottom": pad_v,
            "layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG", "cornerRadius": radius,
            "fills": [solid(*PRIMARY, var="VariableID:1:1")],
            "effects": [{"type": "DROP_SHADOW", "visible": True, "color": {"r": 0, "g": 0, "b": 0, "a": 0.1},
                         "offset": {"x": 0, "y": 1}, "radius": 3, "spread": 0}],
            "styles": {"effect": "S:shadow"},
            "boundVariables": {"itemSpacing": alias("VariableID:1:4")},
            "children": [
                icon_instance(icon_id, x + pad_h, pad_v, refs={"visible": "Show icon#1:1", "mainComponent": "Icon#1:2"}, visible=False),
                text(label_id, "Label", "Button", x + pad_h + 24, pad_v, 60, 24, size=size, weight=600,
                     fill=solid(*WHITE, var="VariableID:1:3"), style_id="S:body", refs={"characters": "Label#1:0"}),
            ]}

components_page = {"id": "1:0", "name": "Components", "type": "CANVAS", "children": [
    {"id": "1:10", "name": "Button", "type": "COMPONENT_SET", "absoluteBoundingBox": bb(0, 0, 300, 48),
     "componentPropertyDefinitions": {
         "Size": {"type": "VARIANT", "defaultValue": "Large", "variantOptions": ["Large", "Small"]},
         "Label#1:0": {"type": "TEXT", "defaultValue": "Button"},
         "Show icon#1:1": {"type": "BOOLEAN", "defaultValue": False},
         "Icon#1:2": {"type": "INSTANCE_SWAP", "defaultValue": "1:30", "preferredValues": []}},
     "children": [
         button_variant("1:11", "Size=Large", 0, 12, 20, 8, 16, "1:12", "1:13"),
         button_variant("1:14", "Size=Small", 150, 6, 12, 6, 14, "1:15", "1:16"),
     ]},
    {"id": "1:30", "name": "Icon/Check", "type": "COMPONENT", "absoluteBoundingBox": bb(0, 100, 16, 16),
     "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "FIXED",
     "children": [vector("1:31", 2, 104)]},
    {"id": "1:40", "name": "Todo row", "type": "COMPONENT", "absoluteBoundingBox": bb(0, 200, 360, 48),
     "componentPropertyDefinitions": {
         "Title#2:0": {"type": "TEXT", "defaultValue": "Buy milk"},
         "Done#2:1": {"type": "BOOLEAN", "defaultValue": False}},
     "layoutMode": "HORIZONTAL", "counterAxisAlignItems": "CENTER", "itemSpacing": 12,
     "paddingLeft": 16, "paddingRight": 16, "paddingTop": 12, "paddingBottom": 12,
     "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "HUG",
     "strokes": [solid(*BORDER)], "strokeWeight": 1, "strokeAlign": "INSIDE",
     "children": [
         {"id": "1:41", "name": "Checkbox", "type": "ELLIPSE", "absoluteBoundingBox": bb(16, 214, 20, 20),
          "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "FIXED",
          "fills": [], "strokes": [solid(*GRAY)], "strokeWeight": 2},
         text("1:42", "Title", "Buy milk", 48, 212, 260, 24, refs={"characters": "Title#2:0"},
              extra={"layoutSizingHorizontal": "FILL", "textAutoResize": "HEIGHT"}),
         dict(icon_instance("1:43", 330, 216, refs={"visible": "Done#2:1"}, visible=False), name="Done icon"),
     ]},
]}

import copy

def find(node, id_):
    if node.get("id") == id_: return node
    for c in node.get("children", []):
        r = find(c, id_)
        if r: return r
    return None

def reid(node, inst):
    node["id"] = "I" + inst + ";" + node["id"].lstrip("I")
    for c in node.get("children", []): reid(c, inst)

def instance_of(comp_id, inst_id, name, bbox, props, extra=None):
    """A real-API-shaped instance: a deep copy of the main component."""
    main = copy.deepcopy(find(components_page, comp_id))
    main.pop("componentPropertyDefinitions", None)
    for c in main.get("children", []): reid(c, inst_id)
    main.update({"id": inst_id, "name": name, "type": "INSTANCE", "componentId": comp_id,
                 "absoluteBoundingBox": bbox, "componentProperties": props})
    if extra: main.update(extra)
    return main

add_instance = instance_of("1:14", "2:5", "Add", bb(1300, 96, 70, 36), {
        "Size": {"type": "VARIANT", "value": "Small"},
        "Label#1:0": {"type": "TEXT", "value": "Add"},
        "Show icon#1:1": {"type": "BOOLEAN", "value": True},
        "Icon#1:2": {"type": "INSTANCE_SWAP", "value": "1:30", "preferredValues": []}},
    extra={"layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG"})
# The instance shows its icon and its own label text.
icon = find(add_instance, "I2:5;1:15"); icon["visible"] = True
find(add_instance, "I2:5;1:16")["characters"] = "Add"

def row_instance(id_, title, done, y):
    row = instance_of("1:40", id_, "Todo row", bb(32, y, 1376, 48), {
        "Title#2:0": {"type": "TEXT", "value": title},
        "Done#2:1": {"type": "BOOLEAN", "value": done}},
        extra={"layoutSizingHorizontal": "FILL", "layoutSizingVertical": "HUG"})
    find(row, "I" + id_ + ";1:42")["characters"] = title
    find(row, "I" + id_ + ";1:43")["visible"] = done
    return row

screens_page = {"id": "2:0", "name": "Screens", "type": "CANVAS", "children": [
    {"id": "2:1", "name": "Todos", "type": "FRAME", "absoluteBoundingBox": bb(0, 0, 1440, 900),
     "layoutMode": "VERTICAL", "itemSpacing": 16, "paddingLeft": 32, "paddingRight": 32, "paddingTop": 32, "paddingBottom": 32,
     "fills": [solid(1, 1, 1, var="VariableID:1:2")], "clipsContent": True,
     "children": [
         text("2:2", "Title", "My todos", 32, 32, 200, 40, size=32, weight=700, lh=40, style_id="S:heading"),
         {"id": "2:3", "name": "New todo", "type": "FRAME", "absoluteBoundingBox": bb(32, 88, 1376, 48),
          "layoutMode": "HORIZONTAL", "itemSpacing": 8, "counterAxisAlignItems": "CENTER",
          "layoutSizingHorizontal": "FILL", "layoutSizingVertical": "HUG",
          "children": [
              {"id": "2:4", "name": "input", "type": "FRAME", "absoluteBoundingBox": bb(32, 88, 1290, 40),
               "layoutMode": "HORIZONTAL", "paddingLeft": 12, "paddingRight": 12, "paddingTop": 8, "paddingBottom": 8,
               "layoutSizingHorizontal": "FILL", "layoutSizingVertical": "HUG", "cornerRadius": 6,
               "strokes": [solid(*BORDER)], "strokeWeight": 1,
               "fills": [solid(1, 1, 1)],
               "children": [text("2:41", "Placeholder", "What needs doing?", 44, 96, 200, 24, fill=solid(*GRAY))]},
              add_instance,
          ]},
         {"id": "2:6", "name": "List", "type": "FRAME", "absoluteBoundingBox": bb(32, 152, 1376, 96),
          "layoutMode": "VERTICAL", "itemSpacing": 0, "layoutSizingHorizontal": "FILL", "layoutSizingVertical": "HUG",
          "children": [row_instance("2:7", "Buy milk", True, 152), row_instance("2:8", "Walk the dog", False, 200)]},
         text("2:9", "Count", "2 items left", 32, 264, 100, 20, size=14, lh=20, fill=solid(*GRAY)),
         {"id": "2:10", "name": "Hero art", "type": "FRAME", "absoluteBoundingBox": bb(32, 300, 400, 200),
          "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "FIXED", "cornerRadius": 12, "clipsContent": True,
          "children": [
              {"id": "2:11", "name": "Photo", "type": "RECTANGLE", "absoluteBoundingBox": bb(32, 300, 400, 200),
               "fills": [{"type": "IMAGE", "imageRef": "abc123", "scaleMode": "FILL"}]},
              text("2:12", "Caption", "Ship it\nToday", 48, 440, 120, 48, size=16, weight=700,
                   fill=solid(*WHITE), extra={"layoutPositioning": "AUTO"}),
          ]},
         {"id": "2:13", "name": "Divider", "type": "RECTANGLE", "visible": False, "absoluteBoundingBox": bb(32, 520, 1376, 1)},
         {"id": "2:14", "name": "Map embed", "type": "EMBED", "absoluteBoundingBox": bb(32, 530, 300, 200)},
     ]},
    {"id": "2:20", "name": "Note", "type": "TEXT", "characters": "Not a frame", "absoluteBoundingBox": bb(2000, 0, 100, 20)},
]}

draft_page = {"id": "3:0", "name": "Drafts", "type": "CANVAS", "children": [
    {"id": "3:1", "name": "Scratch", "type": "FRAME", "absoluteBoundingBox": bb(0, 0, 100, 100), "children": []}]}

doc = {"name": "Todo", "version": "4242", "lastModified": "2026-10-01T00:00:00Z", "role": "owner",
       "schemaVersion": 0, "thumbnailUrl": "https://example.invalid/thumb.png",
       "document": {"id": "0:0", "name": "Document", "type": "DOCUMENT", "children": [components_page, screens_page, draft_page]},
       "components": {
           "1:11": {"key": "k11", "name": "Size=Large", "description": "", "componentSetId": "1:10", "remote": False, "documentationLinks": []},
           "1:14": {"key": "k14", "name": "Size=Small", "description": "", "componentSetId": "1:10", "remote": False, "documentationLinks": []},
           "1:30": {"key": "k30", "name": "Icon/Check", "description": "", "remote": False, "documentationLinks": []},
           "1:40": {"key": "k40", "name": "Todo row", "description": "", "remote": False, "documentationLinks": []}},
       "componentSets": {"1:10": {"key": "k10", "name": "Button", "description": "", "remote": False}},
       "styles": {
           "S:heading": {"key": "sh", "name": "Heading/H1", "styleType": "TEXT", "description": "", "remote": False},
           "S:body": {"key": "sb", "name": "Body/Strong", "styleType": "TEXT", "description": "", "remote": False},
           "S:shadow": {"key": "ss", "name": "Shadow/Card", "styleType": "EFFECT", "description": "", "remote": False}}}

def c(r, g, b, a=1): return {"r": r, "g": g, "b": b, "a": a}
variables = {"status": 200, "error": False, "meta": {
    "variableCollections": {
        "VariableCollectionId:1:0": {"id": "VariableCollectionId:1:0", "name": "Theme", "key": "t", "defaultModeId": "1:0",
            "modes": [{"modeId": "1:0", "name": "Light"}, {"modeId": "1:1", "name": "Dark"}], "remote": False,
            "hiddenFromPublishing": False, "variableIds": ["VariableID:1:1", "VariableID:1:2", "VariableID:1:3"]},
        "VariableCollectionId:2:0": {"id": "VariableCollectionId:2:0", "name": "Primitives", "key": "p", "defaultModeId": "2:0",
            "modes": [{"modeId": "2:0", "name": "Value"}], "remote": False, "hiddenFromPublishing": False,
            "variableIds": ["VariableID:1:4", "VariableID:1:5"]}},
    "variables": {
        "VariableID:1:1": {"id": "VariableID:1:1", "name": "color/brand/primary", "key": "a", "variableCollectionId": "VariableCollectionId:1:0",
            "resolvedType": "COLOR", "valuesByMode": {"1:0": c(*PRIMARY), "1:1": c(0.655, 0.545, 0.98)}, "remote": False,
            "description": "", "hiddenFromPublishing": False, "scopes": ["ALL_FILLS"], "codeSyntax": {}},
        "VariableID:1:2": {"id": "VariableID:1:2", "name": "color/surface", "key": "b", "variableCollectionId": "VariableCollectionId:1:0",
            "resolvedType": "COLOR", "valuesByMode": {"1:0": c(1, 1, 1), "1:1": c(0.067, 0.094, 0.153)}, "remote": False,
            "description": "", "hiddenFromPublishing": False, "scopes": ["ALL_FILLS"], "codeSyntax": {}},
        "VariableID:1:3": {"id": "VariableID:1:3", "name": "color/on-primary", "key": "c", "variableCollectionId": "VariableCollectionId:1:0",
            "resolvedType": "COLOR", "valuesByMode": {"1:0": alias("VariableID:1:2"), "1:1": c(1, 1, 1)}, "remote": False,
            "description": "", "hiddenFromPublishing": False, "scopes": ["TEXT_FILL"], "codeSyntax": {}},
        "VariableID:1:4": {"id": "VariableID:1:4", "name": "spacing/sm", "key": "d", "variableCollectionId": "VariableCollectionId:2:0",
            "resolvedType": "FLOAT", "valuesByMode": {"2:0": 8}, "remote": False,
            "description": "", "hiddenFromPublishing": False, "scopes": ["GAP"], "codeSyntax": {}},
        "VariableID:1:5": {"id": "VariableID:1:5", "name": "font/weight/bold", "key": "e", "variableCollectionId": "VariableCollectionId:2:0",
            "resolvedType": "FLOAT", "valuesByMode": {"2:0": 700}, "remote": False,
            "description": "", "hiddenFromPublishing": False, "scopes": ["FONT_WEIGHT"], "codeSyntax": {}}}}}

with open(out + "/snapshot.json", "w") as f: json.dump(doc, f, indent=2, sort_keys=True); f.write("\n")
with open(out + "/variables.json", "w") as f: json.dump(variables, f, indent=2, sort_keys=True); f.write("\n")
