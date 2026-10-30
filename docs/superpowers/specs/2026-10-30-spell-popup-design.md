# Infobulle de correction au survol d'un mot souligné

Date : 2026-10-30

## Intention

Dans Claude Code, le mod [ortho](https://github.com/yrbane/claude-ortho) souligne
en rouge (orthographe) et en bleu (grammaire) les fautes de la zone de saisie,
grâce à un serveur LanguageTool local. Le moteur de Claude Code ne transmet
aucun événement de souris sur cette zone à un mod : le survol ne peut donc pas
venir du mod. Il vient du terminal : rustty voit la souris, les cellules et
leurs attributs.

## Ce que fait rustty

1. **Détection.** Quand la souris s'arrête sur une cellule d'un panneau portant
   l'attribut `UNDERLINE`, rustty étend la plage aux cellules soulignées
   contiguës de la même ligne : c'est le **mot survolé**. La ligne visible
   entière (texte des cellules, continuations de caractères larges exclues)
   sert de **contexte**.
2. **Requête.** Rustty interroge LanguageTool (`POST {url}/v2/check`, `text` =
   contexte, `language=auto`, `preferredVariants=en-US`) sur un thread, pour ne
   jamais bloquer l'interface. Le résultat revient par `UserEvent::SpellResult`.
   La correspondance retenue est celle qui recouvre le mot survolé (offsets en
   unités UTF-16, comme LanguageTool). Pas de correspondance : rien ne s'affiche.
   Serveur injoignable : pas de nouvelle requête pendant 30 s.
3. **Infobulle.** Une petite boîte à bordure, même style que le menu du clic
   droit, s'affiche au-dessus du mot (en dessous s'il n'y a pas la place) :
   le message de LanguageTool, puis jusqu'à 6 remplacements, survolables et
   cliquables. Elle disparaît dès que la souris quitte le mot et la boîte, que
   le panneau se redessine avec un autre contenu sous la souris, ou sur une
   touche.
4. **Application.** Un clic sur un remplacement envoie au programme du panneau
   un **jeton** (voir contrat), encadré par les marques de collage entre
   crochets si le mode est actif. C'est le mod ortho qui applique la
   correction dans le brouillon ; rustty ne déplace jamais le curseur.

## Contrat rustty → mod ortho

Le jeton est une suite de caractères de la zone à usage privé, que le mod
consomme dans son hook `prompt.edit` sans rien laisser dans le brouillon :

```
U+E000  "ortho"  U+E001  <mot>  U+E001  <remplacement>  U+E001  <occurrence>  U+E002
```

- `<mot>` : le texte souligné survolé, tel que lu dans les cellules ;
- `<remplacement>` : la suggestion choisie ;
- `<occurrence>` : rang (à partir de 0) de ce mot parmi les plages soulignées
  de même texte dans le panneau, de haut en bas puis de gauche à droite. Le mod
  l'utilise pour choisir la bonne correspondance quand le même mot est fautif
  plusieurs fois dans le brouillon.

Le mod remplace `[start, end)` de la correspondance visée par le remplacement,
replace le curseur s'il était après, et relance une vérification.

Hors Claude Code (ou sans le mod), un clic insérerait le jeton tel quel : la
boîte ne s'affiche que sur un mot souligné pour lequel LanguageTool a une
correspondance, ce qui n'arrive en pratique que dans la saisie corrigée.

## Configuration

```toml
[spellcheck]
# Infobulle de correction au survol d'un mot souligné (LanguageTool local).
enabled = true
# Serveur LanguageTool interrogé ; celui que lance le mod ortho de Claude Code.
url = "http://127.0.0.1:8099"
```

## Non-objectifs

- Pas de vérification orthographique par rustty lui-même : il ne regarde que
  ce qu'un programme a déjà souligné.
- Pas d'application par touches de curseur (fragile dans un brouillon replié).
- Pas de délai de survol configurable : la requête part dès l'entrée sur un
  mot et l'infobulle suit la réponse.

## Laissé au constructeur

Découpage des modules, placement exact (marges, largeur maximale), rendu des
textes trop longs (coupés avec `…` comme les libellés de menu).
