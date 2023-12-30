To generate the POT file (from `src/gui/slint`):

Powershell:
```powershell
slint-tr-extractor @(gci -r -fi "*.slint") -o localization\spc-presenter-rs.pot
```

Bash:
```bash
find -name \*.slint | xargs slint-tr-extractor -o localization\spc-presenter-rs.pot
```
