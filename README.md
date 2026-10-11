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

- Une héroïne contrôlée en **mouvement libre relatif à la caméra** : le stick ou WASD indique une direction, pas une rotation. Quand la caméra change de plan, la direction tenue est conservée.
- Un manoir de 5 pièces : hall, salle à manger, couloir est, bureau et bibliothèque. Chaque pièce a plusieurs **caméras fixes**, et le passage d'une pièce à l'autre se fait par la **porte qui s'ouvre** comme dans la série.
- Par les fenêtres : ciel étoilé, **lune** voilée de nuages, **montagnes**, marais brumeux avec roseaux, arbres morts, grille en fer forgé et lucioles. Des **chiens aboient** au loin, et parfois leurs yeux luisent dans le noir. On entend aussi des grenouilles et le vent.
- Objets : **clé au blason** (ouvre la bibliothèque), **trousses de soins** et **munitions limitées** (chargeur de 15).
- **Zombies** : l'un dévore un cadavre, les autres errent. Ils vous saisissent et vous mordent. Le tir verrouille automatiquement la cible, avec une chance de tir critique.
- Le **checkpoint** se fait à la machine à écrire du hall. Il est **conservé sur disque** et se reprend avec « Continuer » sur l'écran titre. En cas de mort, la partie reprend à la dernière sauvegarde.
- L'inventaire affiche l'**électrocardiogramme** (Bien / Attention / Danger). En danger, l'héroïne boite.

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
| Tirer       | Clic gauche, Espace ou F    | RT (ou RB)         |
| Recharger   | R                           | X                  |
| Action / ramasser / ouvrir | E ou Entrée  | A                  |
| Inventaire  | Tab ou I                    | Y / Affichage      |
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
cargo test --release                       # tests de gameplay (portes, clé, tir, morsure, soins, sauvegarde)
cargo run --release -- --shots captures/   # captures d'écran (pièces, menus clavier/manette) + temps de rendu
cargo run --release -- --visibility        # vérifie le cadrage : joueuse, portes et objets visibles, aucun trou
cargo run --release -- --shot 2 7.5 3 x.ppm  # capture : pièce, x, z de la joueuse
cargo run --release -- --model modele/      # planche du modèle de l'héroïne (face, dos, visée, course, visage)
```

| Fichier          | Rôle |
|------------------|------|
| `src/raster.rs`  | Rasteriseur PS1 (snapping, affine, tramage, brouillard, texte 8×8) |
| `src/mesh.rs`    | Maillages et primitives de construction |
| `src/textures.rs`| Textures procédurales |
| `src/world.rs`   | Pièces, portes, caméras fixes, lumières, objets |
| `src/outdoor.rs` | Marais, montagnes, ciel, lune |
| `src/actors.rs`  | Modèles articulés et animations procédurales |
| `src/game.rs`    | Logique de jeu, IA, diagnostic de cadrage |
| `src/game/menu.rs` | Menus, options, interface, icônes clavier/manette |
| `src/game/save.rs` | Format de la sauvegarde |
| `src/options.rs` | Options, stockage persistant, requêtes à la plateforme |
| `src/audio.rs`   | Synthèse des sons et mixeur |
| `src/input.rs`   | Clavier, souris, manette (gilrs) |
| `src/native.rs`  | Frontend natif (winit + softbuffer + cpal) |
| `src/web.rs`     | Frontend WASM (canvas 2D + WebAudio) |
