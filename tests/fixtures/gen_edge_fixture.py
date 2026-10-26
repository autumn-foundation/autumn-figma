"""Write the edge fixture: hostile and unusual designs that must still
generate code that compiles under pedantic clippy.

Run: python3 -I tests/fixtures/gen_edge_fixture.py tests/fixtures/edge/figma
"""
import json, sys

def bb(x, y, w, h): return {"x": x, "y": y, "width": w, "height": h}
def solid(r, g, b): return {"type": "SOLID", "color": {"r": r, "g": g, "b": b, "a": 1}}
def text(id_, name, chars, refs=None):
    n = {"id": id_, "name": name, "type": "TEXT", "characters": chars, "absoluteBoundingBox": bb(0, 0, 80, 20),
         "style": {"fontFamily": "Inter", "fontSize": 14, "fontWeight": 400, "lineHeightPx": 20,
                   "lineHeightUnit": "PIXELS", "textAutoResize": "WIDTH_AND_HEIGHT"},
         "layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG", "fills": [solid(0, 0, 0)]}
    if refs: n["componentPropertyReferences"] = refs
    return n

def variant(id_, name):
    return {"id": id_, "name": name, "type": "COMPONENT", "layoutMode": "HORIZONTAL",
            "absoluteBoundingBox": bb(0, 0, 100, 32), "layoutSizingHorizontal": "HUG",
            "layoutSizingVertical": "HUG", "fills": [solid(0.2, 0.4, 0.9)],
            "children": [text(id_ + "t", "Label", name)]}

components = {"id": "1:0", "name": "Kit ‮<b>", "type": "CANVAS", "children": [
    # Variant values that share a suffix trigger clippy::enum_variant_names.
    {"id": "1:1", "name": "Kind", "type": "COMPONENT_SET", "absoluteBoundingBox": bb(0, 0, 400, 40),
     "componentPropertyDefinitions": {"Kind": {"type": "VARIANT", "defaultValue": "Primary Button",
        "variantOptions": ["Primary Button", "Secondary Button", "Tertiary Button"]}},
     "children": [variant("1:2", "Kind=Primary Button"), variant("1:3", "Kind=Secondary Button"),
                  variant("1:4", "Kind=Tertiary Button")]},
    # Two options, one component: the match needs a wildcard arm.
    {"id": "1:5", "name": "Toggle", "type": "COMPONENT_SET", "absoluteBoundingBox": bb(0, 50, 100, 40),
     "componentPropertyDefinitions": {"State": {"type": "VARIANT", "defaultValue": "On", "variantOptions": ["On", "Off"]}},
     "children": [variant("1:6", "State=On")]},
    # No options and no `Key=` names: no enum, one body.
    {"id": "1:7", "name": "Odd \"quoted\" {set}", "type": "COMPONENT_SET", "absoluteBoundingBox": bb(0, 100, 100, 40),
     "componentPropertyDefinitions": {"Mode": {"type": "VARIANT", "defaultValue": "x"}},
     "children": [variant("1:8", "no equals")]},
    # A set without variants renders nothing.
    {"id": "1:9", "name": "Empty", "type": "COMPONENT_SET", "absoluteBoundingBox": bb(0, 150, 10, 10),
     "componentPropertyDefinitions": {"Size": {"type": "VARIANT", "defaultValue": "S", "variantOptions": ["S"]}},
     "children": []},
    # A slot keeps its column box.
    {"id": "1:10", "name": "Card", "type": "COMPONENT", "layoutMode": "VERTICAL", "itemSpacing": 8,
     "absoluteBoundingBox": bb(0, 200, 300, 200), "layoutSizingHorizontal": "FIXED", "layoutSizingVertical": "HUG",
     "componentPropertyDefinitions": {"Body#9:0": {"type": "SLOT", "defaultValue": ""},
                                      "Title#9:1": {"type": "TEXT", "defaultValue": "Card"}},
     "children": [text("1:11", "Title", "Card", {"characters": "Title#9:1"}),
                  {"id": "1:12", "name": "Body", "type": "SLOT", "layoutMode": "VERTICAL", "itemSpacing": 12,
                   "paddingLeft": 16, "paddingRight": 16, "paddingTop": 16, "paddingBottom": 16,
                   "absoluteBoundingBox": bb(0, 230, 300, 100), "layoutSizingHorizontal": "FILL",
                   "layoutSizingVertical": "HUG", "componentPropertyReferences": {"slotContentId": "Body#9:0"},
                   "children": []},
                  {"id": "5:3", "name": "Badge", "type": "INSTANCE", "componentId": "99:9",
                   "absoluteBoundingBox": bb(0, 340, 60, 20), "layoutSizingHorizontal": "HUG",
                   "layoutSizingVertical": "HUG", "layoutMode": "HORIZONTAL",
                   "children": [text("I5:3;4:2", "Label", "Default")]}]},
]}

screens = {"id": "2:0", "name": "Main", "type": "CANVAS", "children": [
    {"id": "2:1", "name": "Home", "type": "FRAME", "layoutMode": "VERTICAL", "absoluteBoundingBox": bb(0, 0, 800, 600),
     "children": [
        text("2:2", "A", "Not a link"),
        text("2:3", "Email", "you@example.com"),
        {"id": "2:4", "name": "Details", "type": "FRAME", "absoluteBoundingBox": bb(0, 40, 800, 40),
         "layoutMode": "HORIZONTAL", "layoutSizingHorizontal": "FILL", "layoutSizingVertical": "HUG",
         "children": [text("2:5", "Body", "Shown")]},
        {"id": "2:6", "name": "Wide card", "type": "INSTANCE", "componentId": "1:10", "layoutMode": "VERTICAL",
         "absoluteBoundingBox": bb(0, 100, 800, 200), "layoutSizingHorizontal": "FILL", "layoutSizingVertical": "HUG",
         "componentProperties": {"Title#9:1": {"type": "TEXT", "value": "Hello"},
                                 "Body#9:0": {"type": "SLOT", "value": {"guid": {"sessionID": 1, "localID": 2}}}},
         "children": [text("I2:6;1:11", "Title", "Hello", {"characters": "Title#9:1"}),
                      {"id": "I2:6;1:12", "name": "Body", "type": "SLOT", "layoutMode": "VERTICAL",
                       "absoluteBoundingBox": bb(0, 130, 800, 60), "componentPropertyReferences": {"slotContentId": "Body#9:0"},
                       "children": [text("I2:6;1:12;7:1", "Line", "Slot content")]},
                      {"id": "I2:6;5:3", "name": "Badge", "type": "INSTANCE", "componentId": "99:9",
                       "absoluteBoundingBox": bb(0, 240, 60, 20), "layoutMode": "HORIZONTAL",
                       "children": [text("I2:6;5:3;4:2", "Label", "NEW")]}]},
        {"id": "2:7", "name": "Toggle", "type": "INSTANCE", "componentId": "1:6", "layoutMode": "HORIZONTAL",
         "absoluteBoundingBox": bb(0, 320, 100, 32), "layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG",
         "componentProperties": {"State": {"type": "VARIANT", "value": "On"}},
         "children": [text("I2:7;1:6t", "Label", "State=On")]},
     ]},
    {"id": "2:10", "name": "Overlay", "type": "FRAME", "absoluteBoundingBox": bb(1000, 0, 400, 400), "children": [
        {"id": "2:11", "name": "Pinned", "type": "INSTANCE", "componentId": "1:2", "layoutMode": "HORIZONTAL",
         "absoluteBoundingBox": bb(1200, 150, 100, 32),
         "componentProperties": {"Kind": {"type": "VARIANT", "value": "Secondary Button"}},
         "children": [text("I2:11;1:2t", "Label", "Kind=Secondary Button")]}]},
    {"id": "2:20", "name": "Grouped", "type": "GROUP", "absoluteBoundingBox": bb(0, 900, 100, 100), "children": [
        {"id": "2:21", "name": "In group", "type": "COMPONENT", "absoluteBoundingBox": bb(0, 900, 100, 40),
         "children": [text("2:22", "T", "inside a group")]}]},
]}

reserved = {"id": "3:0", "name": "rustfmt", "type": "CANVAS", "children": [
    {"id": "3:1", "name": "Fmt", "type": "FRAME", "absoluteBoundingBox": bb(0, 0, 10, 10), "children": []}]}

doc = {"name": "Edge ⁦cases", "version": "7", "document": {"id": "0:0", "type": "DOCUMENT",
       "children": [components, screens, reserved]}, "components": {}, "styles": {}}

out = sys.argv[1]
with open(out + "/snapshot.json", "w") as f: json.dump(doc, f, indent=2, sort_keys=True); f.write("\n")
