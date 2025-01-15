# Machiene Readable EXIF tags

This directory contains EXIF tag information in a machiene-readable form.
The data in this directory is extracted from [an internet-archive snapshot of tiki-lounge](https://web.archive.org/web/20120202141457/http://www.tiki-lounge.com/~raf/tiff/fields.html)
and then converted using a combination of [this web tool](https://www.convertjson.com/html-table-to-json.htm),
some hacked-together JS (see below) and manual labour.

## Convert JS:

```js
input.map((x) => {
  function mapType(typeString) {
    const dtype_string = typeString.match(/^[A-Z\n]*/g)[0];
    const dtype = dtype_string.split("\n").slice(0, -1);
    const kind = typeString.match(/[A-Z]*$/g)[0];

    const rest_str = typeString
      .substr(
        dtype_string.length,
        typeString.length - dtype_string.length - kind.length,
      )
      .trim();

    let rest = {};
    if (kind == "BITFLAGS") {
      const values = Object.fromEntries(
        rest_str.split("\n").map((x) =>
          x
            .split(":")
            .map((x) => x.trim())
            .reverse(),
        ),
      );
      rest = { values };
    } else if (kind == "ENUMERATED") {
      const values = Object.fromEntries(
        rest_str.split("\n").map((x) =>
          x
            .split("=")
            .map((x) => x.trim())
            .reverse(),
        ),
      );
      rest = { values };
    } else {
      if (rest_str) {
        rest = {
          rest: rest_str,
        };
      }
    }

    return {
      kind,
      dtype,
      ...rest,
    };
  }

  return {
    ...x,
    name: x.name.replace(/(<([^>]+)>)/gi, "").trim(),
    type: mapType(x.type),
  };
});
```
