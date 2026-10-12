# Le Manoir des Marais

Survival horror façon PlayStation 1 (caméras fixes, ambiance Resident Evil), écrit en Rust.
Le rendu est **entièrement logiciel** et imite le GPU de la PS1 :

- résolution interne 320×240, mise à l'échelle sans lissage ;
- sommets « snappés » au pixel (tremblement des polygones) ;
- textures affines sans correction de perspective (ondulation caractéristique) ;
- éclairage de Gouraud par sommet, brouillard par sommet ;
- couleurs 15 bits avec tramage ordonné 4×4.

Les textures, modèles et sons sont tous **générés procéduralement** au démarrage : il n'y a aucun fichier d'asset.

## Contenu de cette version

Un niveau complet, du hall du manoir jusqu'au marais sous la pleine lune.

- Une héroïne contrôlée en **mouvement libre relatif à la caméra** : le stick ou WASD indique une direction, pas une rotation. Quand la caméra change de plan, la direction tenue est conservée.
- **14 lieux** à caméras fixes :
  - **rez-de-chaussée** : hall, salle à manger, couloir est, bureau, bibliothèque (avec son escalier vers l'étage) et **cuisine** ;
  - **étage** : **galerie des portraits**, **chambre des maîtres**, **chambre d'Alice**, **laboratoire** du Docteur ;
  - **sous-sol** : **escalier de la crypte** et **crypte des Ashcroft** ;
  - **dehors** : **le parc** devant la façade du manoir, puis **le marais** jusqu'au ponton.
- Transitions animées : porte qui s'ouvre, **escalier** qu'on gravit ou descend, **grille du parc**.
- Par les fenêtres : ciel étoilé, **lune**, **montagnes**, marais brumeux avec roseaux, arbres morts, grille en fer forgé et lucioles (vus de plus haut depuis l'étage). Dehors, le même paysage devient jouable : chemin de terre, eau où l'on patauge (plus lentement), ponton.
- **Armes** : pistolet 9 mm (chargeur de 15), **fusil à pompe** (6 cartouches, dévastateur de près, inutile de loin) et **couteau** (silencieux, inusable : il y a toujours un moyen de s'en sortir). On change d'arme dans l'inventaire.
- **Soins** : trousses (santé complète) et **herbes médicinales** (+35).
- **Ennemis** : zombies (certains dévorent un cadavre, d'autres font les morts et se relèvent), **chiens du marais** qui contournent leur proie et bondissent.
- **Trois boss**, aux attaques télégraphiées et esquivables :
  - le **Sujet 7**, qui brise sa cuve au laboratoire : coups de griffe et charges qui projettent au sol (s'il percute un mur, il reste sonné) ;
  - **l'Abomination**, le Docteur Ashcroft fondu dans près de quatre mètres de chair, dans la crypte : griffe, lame d'os, écrasement à deux bras, et des rugissements qui **découvrent son cœur** (dégâts triplés). Plus rapide à mi-vie. Il porte la clé de la porte d'entrée ;
  - **le Molosse**, chef de meute géant, qui garde le ponton.
- Cinématiques d'apparition des boss (passables), barre de vie et musique de combat.
- **Énigme** des portraits de la galerie, documents à lire (journaux, lettres, notes), une **boîte à musique**.
- Un PNJ : **Edgar**, le vieux majordome blessé, réfugié dans la cuisine. Il aide... tant qu'il est en vie.
- **Économie serrée** : de quoi venir à bout de tout, à condition de ne pas gaspiller ; éviter les zombies est souvent plus sage.
- Trois **machines à écrire** (hall, galerie, escalier de la crypte). Le checkpoint est **conservé sur disque** et se reprend avec « Continuer ». En cas de mort, la partie reprend à la dernière sauvegarde.
- L'inventaire affiche l'**électrocardiogramme** (Bien / Attention / Danger). En danger, l'héroïne boite.
- Écran de fin avec temps de jeu, sauvegardes, ennemis abattus et rang.

### Déroulement (attention, révélations)

1. Bureau : la **clé au blason** ouvre la bibliothèque, dont l'escalier mène à l'étage.
2. Les documents (chambre des maîtres, chambre d'Alice, boîte à musique) donnent l'ordre des portraits : **Alice, Brutus, Eleanor**. Le portrait du Docteur pivote et révèle la **clé au corbeau**.
3. Chambre des maîtres : le **fusil à pompe**, au-dessus de la cheminée.
4. Laboratoire : la **clé de la crypte**... qui réveille le Sujet 7. Edgar ne survit pas à la nuit.
5. Cuisine, escalier de la crypte, puis la crypte : l'Abomination. La **clé du manoir** tombe dans la fosse.
6. La porte d'entrée s'ouvre sur le parc, les chiens, le marais, le Molosse... et la barque.

## Menus et options

L'écran titre propose Continuer, Nouvelle partie, Options et Quitter. Le menu pause (Échap / Start) propose Reprendre, Charger la sauvegarde, Options, Commandes, Menu principal et Quitter.

Options (enregistrées automatiquement) :

- **Affichage** : plein écran (par défaut) ou fenêtré ;
- **Résolution** de la fenêtre, de 640×480 à 1920×1440 (natif uniquement) ;
- **Mise à l'échelle** : étirée en 4:3 ou entière (pixels parfaits) ;
- **Volumes** général, effets et ambiance ;
- **Contrôles** modernes ou **classiques** (« tank » : gauche/droite pivotent, haut avance, bas recule, bas + course = demi-tour) ;
- **Afficher les FPS** ;
- **Voir les commandes**.

Les invites à l'écran (« E EXAMINER », « Ⓐ VALIDER »…) affichent les touches du **dernier périphérique utilisé** : touches du clavier, ou boutons Xbox A/B/X/Y, LT/RT et Menu à la manette.

Emplacement des données :

- Linux : `~/.config/manoir-des-marais/`
- Windows : `%APPDATA%\manoir-des-marais\`
- Web : `localStorage` du navigateur

## Commandes

| Action      | Clavier / souris            | Manette            |
|-------------|-----------------------------|--------------------|
| Se déplacer | WASD (ZQSD en AZERTY), flèches | Stick gauche / croix |
| Courir      | Maj                         | Stick à fond ou B  |
| Viser       | Clic droit (suit la souris) ou Ctrl | LT (ou LB)   |
| Tirer / frapper | Clic gauche, Espace ou F | RT (ou RB)         |
| Recharger   | R                           | X                  |
| Action / ramasser / ouvrir | E ou Entrée  | A                  |
| Inventaire (soins, changer d'arme) | Tab ou I | Y / Affichage |
| Pause       | Échap ou P                  | Menu (Start)       |

Les touches sont **physiques** : sur un clavier AZERTY, ZQSD tombe donc à la place de WASD.
En visant au clavier, A et D font pivoter l'héroïne. À la manette, le stick oriente directement la visée.

## Lancer en natif

```sh
cargo run --release
```

Sous Linux, il faut les en-têtes ALSA (`alsa-lib`) et udev (pour la manette).

## Compiler pour le web (WASM)

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # doit correspondre à la version du Cargo.toml
./build-web.sh
python3 -m http.server -d web 8080                 # puis http://localhost:8080
```

Le dossier `web/` (`index.html` + `pkg/`) se déploie tel quel sur n'importe quel hébergement statique.
Les navigateurs n'activent le son qu'après une première touche ou un premier clic.

## Développement

```sh
cargo test --release                       # tests de gameplay (portes, clés, armes, énigme, boss, fin, sauvegarde, économie)
cargo run --release -- --shots captures/   # captures (pièces, cinématiques, combats, menus, fin) + temps de rendu
cargo run --release -- --visibility        # vérifie le cadrage : joueuse, portes et objets visibles, aucun trou
cargo run --release -- --shot 2 7.5 3 x.ppm  # capture : pièce, x, z de la joueuse
cargo run --release -- --model modele/      # planches des modèles : héroïne, Edgar, Sujet 7, chiens, Molosse, Abomination
```

| Fichier          | Rôle |
|------------------|------|
| `src/raster.rs`  | Rasteriseur PS1 (snapping, affine, tramage, brouillard, texte 8×8) |
| `src/mesh.rs`    | Maillages et primitives de construction |
| `src/textures.rs`| Textures procédurales |
| `src/world.rs`   | Pièces du rez-de-chaussée, portes, caméras fixes, lumières, objets, ennemis |
| `src/world/upstairs.rs` | Galerie, chambres, laboratoire |
| `src/world/below.rs` | Cuisine, escalier de la crypte, crypte |
| `src/world/outside.rs` | Parc et marais (pièces en plein air) |
| `src/outdoor.rs` | Terrain, chemin, ponton, façade du manoir, montagnes, ciel, lune |
| `src/actors.rs`  | Modèles articulés (héroïne, zombies, Edgar, Sujet 7, chiens, Abomination) et animations procédurales |
| `src/game.rs`    | Boucle de jeu, joueuse, objets, portes, inventaire, diagnostic de cadrage |
| `src/game/combat.rs` | Armes, visée automatique, dégâts |
| `src/game/enemies.rs` | IA des zombies, chiens et boss |
| `src/game/story.rs` | Drapeaux d'histoire, déclencheurs, cinématiques, énigme, Edgar, fin |
| `src/game/render.rs` | Rendu des pièces, du plein air, des créatures et des transitions |
| `src/game/menu.rs` | Menus, options, interface, icônes clavier/manette |
| `src/game/save.rs` | Format de la sauvegarde |
| `src/options.rs` | Options, stockage persistant, requêtes à la plateforme |
| `src/audio.rs`   | Synthèse des sons et mixeur |
| `src/input.rs`   | Clavier, souris, manette (gilrs) |
| `src/native.rs`  | Frontend natif (winit + softbuffer + cpal) |
| `src/web.rs`     | Frontend WASM (canvas 2D + WebAudio) |
