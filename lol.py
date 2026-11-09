import json
import os
import string

for i in os.listdir("/home/bio/mus"):
    # if i.startswith("."): continue
    if i == "archive.txt":
        continue
    assert i.endswith(".m4a"), i
    i = i[:-4]
    assert i.endswith("]"), i
    assert i[-13] == "["
    id = i[-12:-1]
    i = i[:-13]
    artist = ""
    if "-" in i:
        dash = i.find("-")
        artist = i[:dash].strip()
        i = i[dash + 1 :].strip()

    i = i.split("[")[0]
    i = i.split("(")[0]
    i = i.split("（")[0]
    i = i.split("feat.")[0]
    i = i.split("Feat.")[0]
    i = i.split("FEAT.")[0]
    i = i.strip()
    if artist == "DECO＊27":
        artist='deco27'

    title = json.dumps(i, ensure_ascii=False)
    artist = json.dumps(artist, ensure_ascii=False)
    toml = (
        f"""
title = {title}
artist = {artist}
source.youtube = "{id}"
""".strip()
        + "\n"
    )
    name = (artist + "-" + title).casefold().replace(" ", "-")
    name = "".join(c for c in name if c in string.ascii_letters + string.digits + "-")
    name = name.strip('-')
    if name != "deco27":
        continue
    name = id
    if not name:
        name = id
    with open("instance/" + name + ".toml", 'w') as f:
        print(name)
        f.write(toml)
