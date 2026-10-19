# ReactOS notepad.exe — ressources décodées (0.4.17-dev-1208-ge2a3e76)

Extrait par parseur PE custom. NB : IDs de chaînes du binaire = master notepad_res.h + 0x10.

## Menu MAIN_MENU (0x201) — EN (0x0409)

POPUP '&File'
  id=0x0100 opt=0x0000 '&New\tCtrl+N'
  id=0x0101 opt=0x0000 'New &Window\tCtrl+Shift+N'
  id=0x0102 opt=0x0000 '&Open...\tCtrl+O'
  id=0x0103 opt=0x0000 '&Save\tCtrl+S'
  id=0x0104 opt=0x0000 'Save &As...'
  ---
  id=0x0106 opt=0x0000 'Page Set&up...'
  id=0x0105 opt=0x0000 '&Print...\tCtrl+P'
  ---
  id=0x0108 opt=0x0080 'E&xit'
POPUP '&Edit'
  id=0x0110 opt=0x0000 '&Undo\tCtrl+Z'
  ---
  id=0x0111 opt=0x0000 'Cu&t\tCtrl+X'
  id=0x0112 opt=0x0000 '&Copy\tCtrl+C'
  id=0x0113 opt=0x0000 '&Paste\tCtrl+V'
  id=0x0114 opt=0x0000 'De&lete\tDel'
  ---
  id=0x0120 opt=0x0000 '&Find...\tCtrl+F'
  id=0x0121 opt=0x0000 'Find &Next\tF3'
  id=0x0122 opt=0x0000 '&Replace...\tCtrl+H'
  id=0x0123 opt=0x0000 '&Go To...\tCtrl+G'
  ---
  id=0x0116 opt=0x0000 'Select &All\tCtrl+A'
  id=0x0117 opt=0x0080 'Time/&Date\tF5'
POPUP 'F&ormat'
  id=0x0119 opt=0x0000 '&Word Wrap'
  id=0x0140 opt=0x0080 '&Font...'
POPUP '&View'
  id=0x0150 opt=0x0080 '&Status Bar'
POPUP '&Help'
  id=0x0130 opt=0x0000 'View &Help'
  ---
  id=0x0135 opt=0x0080 '&About Notepad'

## Menu MAIN_MENU (0x201) — DE (0x0007)

POPUP '&Datei'
  id=0x0100 opt=0x0000 '&Neu\tStrg+N'
  id=0x0101 opt=0x0000 'Neues &Fenster\tStrg+Umschalt+N'
  id=0x0102 opt=0x0000 'Ö&ffnen\tStrg+O'
  id=0x0103 opt=0x0000 '&Speichern\tStrg+S'
  id=0x0104 opt=0x0000 'Speichern &unter...'
  ---
  id=0x0106 opt=0x0000 'Seite ein&richten...'
  id=0x0105 opt=0x0000 '&Drucken...\tStrg+P'
  ---
  id=0x0108 opt=0x0080 '&Beenden'
POPUP '&Bearbeiten'
  id=0x0110 opt=0x0000 '&Rückgängig\tStrg+Z'
  ---
  id=0x0111 opt=0x0000 '&Ausschneiden\tStrg+X'
  id=0x0112 opt=0x0000 '&Kopieren\tStrg+C'
  id=0x0113 opt=0x0000 '&Einfügen\tStrg+V'
  id=0x0114 opt=0x0000 '&Löschen\tEntf'
  ---
  id=0x0120 opt=0x0000 'Suchen...\tStrg+F'
  id=0x0121 opt=0x0000 '&Weitersuchen\tF3'
  id=0x0122 opt=0x0000 '&Ersetzen...\tStrg+H'
  id=0x0123 opt=0x0000 'Gehe zu...\tStrg+G'
  ---
  id=0x0116 opt=0x0000 'Alles &markieren\tStrg+A'
  id=0x0117 opt=0x0080 '&Uhrzeit/Datum\tF5'
POPUP 'F&ormat'
  id=0x0119 opt=0x0000 '&Zeilenumbruch'
  id=0x0140 opt=0x0080 '&Schriftart...'
POPUP '&Ansicht'
  id=0x0150 opt=0x0080 'Status&leiste'
POPUP '&Hilfe'
  id=0x0130 opt=0x0000 '&Inhalt'
  ---
  id=0x0135 opt=0x0080 'Inf&o'

## Chaînes EN (0x0409)

- `0x0170` '&f'
- `0x0171` 'Page &p'
- `0x0180` 'Notepad'
- `0x0181` 'ERROR'
- `0x0182` 'WARNING'
- `0x0183` 'Information'
- `0x0184` 'Untitled'
- `0x0185` 'All files (*.*)'
- `0x0186` 'Text files (*.txt)'
- `0x0187` "File '%s' is too large for notepad.\nPlease use a different editor."
- `0x0188` "You didn't enter any text.\nPlease type something and try again"
- `0x0189` "File '%s'\ndoes not exist\n\nDo you want to create a new file?"
- `0x018a` "File '%s'\nhas been modified\n\nWould you like to save the changes?"
- `0x018b` "'%s' can not be found."
- `0x018c` 'Not enough memory to complete this task.\nClose one or more applications to increase the amount of\nfree memory.'
- `0x018d` "Cannot find '%s'"
- `0x0190` 'ANSI'
- `0x0191` 'Unicode'
- `0x0192` 'Unicode (big endian)'
- `0x0193` 'UTF-8'
- `0x0194` 'UTF-8 with BOM'
- `0x0195` 'Windows (CR + LF)'
- `0x0196` 'Unix (LF)'
- `0x0197` 'Mac (CR)'
- `0x0198` 'Line %d, column %d'
- `0x0199` "Cannot print the file '%s'.\n\nMake sure that the printer is turned on and is configured properly."
- `0x019a` 'Lucida Console'
- `0x019b` 'The specified line number is out of range.'
- `0x019c` 'Now printing page...'
- `0x019d` 'The print job is being canceled...'
- `0x019e` 'Printing is successfully done.'
- `0x019f` 'Printing has been canceled.'
- `0x01a0` 'Printing failed.'
- `0x0210` 'Text Document'
- `0x0310` 'Copyright 1997,98 Marcel Baur, 2000 Mike McCormack, 2002 Sylvain Petreolle, 2002 Andriy Palamarchuk\r\n'

## Accélérateurs ID_ACCEL (0x203)

- cmd=0x0116  A  (fVirt=0x09)
- cmd=0x0112  C  (fVirt=0x09)
- cmd=0x0120  F  (fVirt=0x09)
- cmd=0x0123  G  (fVirt=0x09)
- cmd=0x0122  H  (fVirt=0x09)
- cmd=0x0100  N  (fVirt=0x09)
- cmd=0x0101  N  (fVirt=0x0d)
- cmd=0x0102  O  (fVirt=0x09)
- cmd=0x0105  P  (fVirt=0x09)
- cmd=0x0103  S  (fVirt=0x09)
- cmd=0x0113  V  (fVirt=0x09)
- cmd=0x0111  X  (fVirt=0x09)
- cmd=0x0110  Z  (fVirt=0x09)
- cmd=0x0121  F3  (fVirt=0x01)
- cmd=0x0124  F3  (fVirt=0x05)
- cmd=0x0117  F5  (fVirt=0x81)

## Groupe d'icônes IDI_NPICON (0x209)

- membre id=1 : 48x48, 16 couleurs, planes=1, bpp=4, 1640 octets
- membre id=2 : 32x32, 16 couleurs, planes=1, bpp=4, 744 octets
- membre id=3 : 16x16, 16 couleurs, planes=1, bpp=4, 296 octets
- membre id=4 : 48x48, 0 couleurs, planes=1, bpp=8, 3752 octets
- membre id=5 : 32x32, 0 couleurs, planes=1, bpp=8, 2216 octets
- membre id=6 : 16x16, 0 couleurs, planes=1, bpp=8, 1384 octets
- membre id=7 : 48x48, 0 couleurs, planes=1, bpp=32, 9640 octets
- membre id=8 : 32x32, 0 couleurs, planes=1, bpp=32, 4264 octets
- membre id=9 : 16x16, 0 couleurs, planes=1, bpp=32, 1128 octets

`.ico` reconstruit : 25214 octets, md5 `24f05511df2d53a02ec2eec8a4baf3c3`
`notepad.ico` existant : md5 `24f05511df2d53a02ec2eec8a4baf3c3` → IDENTIQUE

## Inventaire des ressources
- type  3  name 0x0001  ×1 langues  ex: 0x409
- type  3  name 0x0002  ×1 langues  ex: 0x409
- type  3  name 0x0003  ×1 langues  ex: 0x409
- type  3  name 0x0004  ×1 langues  ex: 0x409
- type  3  name 0x0005  ×1 langues  ex: 0x409
- type  3  name 0x0006  ×1 langues  ex: 0x409
- type  3  name 0x0007  ×1 langues  ex: 0x409
- type  3  name 0x0008  ×1 langues  ex: 0x409
- type  3  name 0x0009  ×1 langues  ex: 0x409
- type  4  name 0x0201  ×39 langues  ex: 0x7, 0xa, 0xc
- type  5  name 0x0202  ×39 langues  ex: 0x7, 0xa, 0xc
- type  5  name 0x0204  ×39 langues  ex: 0x7, 0xa, 0xc
- type  5  name 0x0207  ×39 langues  ex: 0x7, 0xa, 0xc
- type  5  name 0x020b  ×39 langues  ex: 0x7, 0xa, 0xc
- type  6  name 0x0017  ×39 langues  ex: 0x7, 0xa, 0xc
- type  6  name 0x0018  ×39 langues  ex: 0x7, 0xa, 0xc
- type  6  name 0x0019  ×39 langues  ex: 0x7, 0xa, 0xc
- type  6  name 0x001a  ×39 langues  ex: 0x7, 0xa, 0xc
- type  6  name 0x0021  ×39 langues  ex: 0x7, 0xa, 0xc
- type  6  name 0x0031  ×39 langues  ex: 0x7, 0xa, 0xc
- type  9  name 0x0203  ×39 langues  ex: 0x7, 0xa, 0xc
- type 14  name 0x0209  ×1 langues  ex: 0x409
- type 16  name 0x0001  ×1 langues  ex: 0x409
- type 24  name 0x0001  ×1 langues  ex: 0x409

Langues menu (type 4) : 39 → 0x7 0xa 0xc 0x10 0x13 0x16 0x18 0x1c 0x1d 0x402 0x404 0x405 0x406 0x408 0x409 0x40b 0x40d 0x40e 0x411 0x415 0x416 0x419 0x41a 0x41b 0x41e 0x41f 0x421 0x422 0x424 0x425 0x427 0x42a 0x42b 0x42d 0x439 0x43e 0x443 0x804 0xc04
