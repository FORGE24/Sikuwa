from babel.messages.pofile import read_po
from babel.messages.mofile import write_mo
from pathlib import Path

po_path = Path('i18n/locales/en_US/LC_MESSAGES/sikuwa.po')
mo_path = po_path.with_suffix('.mo')

with open(po_path, 'rb') as f:
    catalog = read_po(f)

with open(mo_path, 'wb') as f:
    write_mo(f, catalog)

print(f"Successfully compiled {po_path} to {mo_path}")
