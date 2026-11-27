# Argos Script — la référence

22 nov. 2026 · v1

## Démarrer

Un script Argos Script s'écrit dans l'éditeur, puis le bouton ▶ « Ajouter à la chart » le trace sur la chart ouverte. Le langage suit Pine Script v6 : un script Pine collé tel quel tourne le plus souvent sans retouche.

Ouvrir l'éditeur :

- PC : « ƒx Indicateurs » → « Mes indicateurs (Argos Script) » → « Modifier mes indicateurs », ou « Importer du Pine Script » pour coller un script venu d'ailleurs.
- Cell : « ƒx » dans la barre du bas → les mêmes lignes.

Premier script, une moyenne mobile réglable :

```
//@version=6
indicator("Ma moyenne", overlay = true)
longueur = input.int(20, "Longueur", minval = 1)
moy = ta.sma(close, longueur)
plot(moy, "Moyenne", color = color.orange, linewidth = 2)
```

Ce qui se passe :

1. Le script tourne une fois par bougie, de la plus ancienne à la plus récente.
2. `input.int` crée un réglage « Longueur » dans la fenêtre ⚙ de l'indicateur.
3. `plot` trace une ligne. `overlay = true` la pose sur les prix ; `false` l'envoie dans un panneau sous la chart.

Le terminal en bas de l'éditeur donne le verdict : « ✅ Compilé », ou « ❌ Ligne N : message » avec la ligne fautive, et la légende du script devient rouge. « ⚠️ Ligne N » est un simple avertissement : le script tourne quand même.

Bon à savoir :

- Toutes les heures sont en UTC.
- Entiers et décimaux sont le même nombre : `int` et `float` ne changent rien au calcul.
- Le bouton { } dans la légende d'un indicateur ouvre son code en lecture seule.

## Le langage

C'est la syntaxe de Pine Script v6 : blocs indentés de 4 espaces, pas d'accolades, une instruction par ligne.

```
//@version=6
indicator("Exemple", overlay = false)
var int compte = 0              // var : gardé d'une bougie à l'autre
hausse = close > close[1]       // [1] = la bougie d'avant
if hausse
    compte += 1
else if close < close[1]
    compte := 0                 // := change une variable déjà déclarée
plot(compte, "Bougies vertes de suite")
```

| Élément | Forme | À savoir |
| --- | --- | --- |
| Déclarer | `x = 1` | `float x = 1` marche aussi, le type écrit est ignoré |
| Changer | `x := 2`, `+=`, `-=`, `*=`, `/=`, `%=` | La variable doit déjà exister |
| Garder entre bougies | `var x = 0` | Calculé une seule fois ; `varip` fait pareil |
| Historique | `close[3]` | Valeur 3 bougies plus tôt ; `na` avant la première |
| Valeur absente | `na`, `na(x)`, `nz(x, 0)`, `fixnan(x)` | `nz` remplace `na` par 0 ou la valeur donnée |
| Conversions | `int`, `float`, `bool`, `str`, `color` | `int` tronque : `int(2.9)` donne 2 |
| Condition | `if` / `else if` / `else`, `switch` | Comme dans Pine |
| Boucles | `for i = 0 to 9 by 2`, `for x in liste`, `for [i, x] in liste`, `while` | `break` et `continue` |
| Fonction | `f(a, b = 1) => a + b` | Sur plusieurs lignes : corps indenté dessous |
| Plusieurs résultats | `[m, s, h] = ta.macd(close, 12, 26, 9)` | Les crochets à gauche reçoivent chaque valeur |
| Types et méthodes | `type Point`, `Point.new(...)`, `method` | Comme Pine v6 |
| Lambdas (Argos) | `(a, b) => a + b` | Pour `array.map`, `filter`, `reduce` |

Opérateurs, du plus faible au plus fort :

| Opérateur | Sens |
| --- | --- |
| `a ? b : c` | Si a alors b sinon c |
| `or`, `and` | En v6, la droite n'est lue que si besoin ; en v5, les deux côtés sont calculés |
| `==`, `!=` | Égal, différent |
| `<`, `<=`, `>`, `>=` | Comparaisons |
| `+`, `-` | `+` colle aussi deux textes : `"RSI " + str.tostring(r)` |
| `*`, `/`, `%` | Une division par 0 donne `na`, jamais une erreur |
| `not`, `-x` | Négation |

Une ligne trop longue continue sur la suivante si elle finit par une parenthèse ouverte ou un opérateur, ou si la suite est décalée d'un nombre d'espaces qui n'est pas un multiple de 4.

Pas encore là : `import` / `export`, `library`, `enum`.

## Les données

Chaque bougie donne ses prix, son volume et son heure ; le script les lit par leur nom, sans rien charger.

| Famille | Noms | À savoir |
| --- | --- | --- |
| Prix | `open`, `high`, `low`, `close`, `volume` | Ceux de la bougie en cours |
| Prix combinés | `hl2`, `hlc3`, `ohlc4`, `hlcc4` | Moyennes toutes faites |
| Numéro | `bar_index`, `last_bar_index` | 0 = la plus ancienne bougie chargée |
| Heure | `time`, `time_close`, `timenow` | En millisecondes UTC ; `timenow` = heure de la dernière bougie ; `time("D")` et `time_close("D")` donnent l'ouverture et la fermeture de la bougie d'une autre unité |
| Morceaux de date | `year`, `month`, `dayofmonth`, `dayofweek`, `hour`, `minute`, `second`, `weekofyear` | En UTC ; `dayofweek` : 1 = dimanche, ou `dayofweek.monday` etc. |
| Construire une date | `timestamp(2026, 11, 22, 14, 30)` | Donne des millisecondes ; le fuseau écrit est ignoré (UTC) |
| État de la bougie | `barstate.isfirst`, `barstate.islast`, `barstate.isrealtime`, `barstate.ishistory`, `barstate.isconfirmed` | `islast` et `isrealtime` sont vrais sur la dernière bougie ; `isnew` est toujours vrai |
| Symbole | `syminfo.ticker`, `syminfo.tickerid`, `syminfo.mintick` | `tickerid` = « BOURSE:symbole » ; `mintick` est déduit des 50 dernières clôtures |
| Symbole (fixes) | `syminfo.currency`, `syminfo.type`, `syminfo.timezone`, `syminfo.pointvalue` | Toujours « USD », « crypto », « Etc/UTC », 1 |
| Unité de temps | `timeframe.period`, `timeframe.multiplier`, `timeframe.in_seconds()` | Par exemple « 60 » pour 1 h |
| Type d'unité | `timeframe.isintraday`, `isdaily`, `isweekly`, `ismonthly`, `isminutes`, `isseconds` | Vrai ou faux |
| Changement d'unité | `timeframe.change("D")` | Vrai sur la première bougie d'un nouveau jour |
| Vrai delta (Argos) | `argos.real_delta` | Achats moins ventes des vrais trades de la bougie |

Exemple : colorer le fond le lundi, et marquer l'ouverture de chaque jour.

```
//@version=6
indicator("Lundis", overlay = true)
bgcolor(dayofweek == dayofweek.monday ? color.new(color.blue, 90) : na)
plotshape(timeframe.change("D"), "Nouveau jour", shape.triangleup, location.belowbar, color.gray)
```

## Les calculs : ta et math

Les fonctions `ta.*` font les calculs d'indicateurs classiques ; chaque appel garde sa propre mémoire, donc il faut l'appeler à chaque bougie, jamais seulement dans un `if`.

```
// Bon : calculé à chaque bougie, utilisé sous condition
r = ta.rsi(close, 14)
signal = r > 70 ? 1 : 0
// Mauvais : ta.rsi ne voit pas toutes les bougies
// if close > open
//     r := ta.rsi(close, 14)
```

Si un appel est dans une condition, l'éditeur affiche un avertissement : « sortez cet appel de cette condition ».

| Fonction | Donne | À savoir |
| --- | --- | --- |
| `ta.sma`, `ta.ema`, `ta.rma`, `ta.wma`, `ta.swma`, `ta.vwma`, `ta.hma` | Moyennes mobiles | `ta.ema` démarre sur une moyenne simple |
| `ta.rsi(src, n)` | RSI 0 à 100 |  |
| `ta.atr(n)`, `ta.tr(handle_na)` | Vraie amplitude | Variable `ta.tr` aussi |
| `ta.stdev`, `ta.variance`, `ta.dev` | Dispersion | `biased = true` par défaut |
| `ta.highest`, `ta.lowest` | Plus haut, plus bas sur n bougies | Source facultative : `ta.highest(n)` lit `high` |
| `ta.highestbars`, `ta.lowestbars` | Combien de bougies en arrière | Négatif ou 0 |
| `ta.change`, `ta.mom`, `ta.roc`, `ta.cum` | Variation, momentum, taux, cumul |  |
| `ta.crossover`, `ta.crossunder`, `ta.cross` | Croisements | Vrai sur la bougie du croisement |
| `ta.rising`, `ta.falling` | Monte ou baisse depuis n bougies |  |
| `ta.barssince`, `ta.valuewhen(cond, src, 0)` | Depuis quand, valeur à ce moment |  |
| `ta.pivothigh`, `ta.pivotlow` | Sommets et creux | `(gauche, droite)` ou `(src, gauche, droite)` ; confirmé après `droite` bougies |
| `ta.macd(src, 12, 26, 9)` | `[macd, signal, histo]` |  |
| `ta.bb(src, 20, 2)`, `ta.bbw` | `[milieu, haut, bas]`, largeur |  |
| `ta.kc(src, n, mult)` | Canal de Keltner | `useTR = true` par défaut |
| `ta.stoch(src, high, low, n)`, `ta.cci`, `ta.mfi` | Oscillateurs |  |
| `ta.dmi(n, lissage)` | `[+DI, -DI, ADX]` |  |
| `ta.supertrend(facteur, période)` | `[ligne, direction]` | Direction -1 = tendance haussière |
| `ta.sar`, `ta.linreg`, `ta.median`, `ta.percentrank`, `ta.range`, `ta.correlation` | Divers |  |
| `ta.vwap(src)` | VWAP du jour | Repart à 0 h UTC ; variable `ta.vwap` sur `hlc3` |
| `ta.obv`, `ta.accdist` | Variables de volume |  |

Pas encore là : `ta.alma`, `ta.wpr`, `ta.kcw`, `ta.cog`, `ta.tsi`, `ta.max`, `ta.min`, `ta.pivot_point_levels`.

`math.*` couvre le reste : `abs`, `ceil`, `floor`, `round(x, décimales)`, `round_to_mintick`, `sqrt`, `exp`, `log`, `log10`, `pow`, `sign`, la trigonométrie (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `todegrees`, `toradians`), `max`, `min`, `avg` (donnent `na` si une valeur manque), `sum(src, n)`, et les constantes `pi`, `e`, `phi`, `rphi`. `math.random(min, max, graine)` donne toujours la même suite pour la même graine.

## Dessiner sur la chart

Deux façons de dessiner : les tracés `plot*` donnent une valeur par bougie, les objets `label`, `line`, `box`, `polyline` et `table` se posent où on veut et se modifient ensuite.

| Fonction | Dessine | À savoir |
| --- | --- | --- |
| `plot(serie, titre, color, linewidth, style)` | Une ligne ou des barres | Styles `plot.style_line`, `linebr`, `stepline`, `histogram`, `columns`, `circles`, `cross`, `area` ; `color = na` saute la bougie ; `display = display.none` cache |
| `hline(prix, titre, color, linestyle)` | Une ligne horizontale fixe | `hline.style_solid`, `dashed`, `dotted` |
| `fill(a, b, color)` | La zone entre deux tracés | Deux `plot` ou deux `hline` |
| `bgcolor(couleur)` | Le fond de la bougie | `na` = pas de fond |
| `barcolor(couleur)` | La couleur de la bougie |  |
| `plotshape(cond, titre, style, location, color, text)` | Un symbole quand la condition est vraie | Par défaut une croix au-dessus de la bougie |
| `plotchar(cond, titre, char)` | Un caractère | Par défaut « ★ » |
| `plotarrow(serie)` | Flèche haut si positif, bas si négatif |  |
| `plotcandle(open, high, low, close, titre, color, wickcolor, bordercolor)` | Des bougies faites de quatre séries | Par exemple des bougies Heikin Ashi |
| `plotbar(open, high, low, close, titre, color)` | Les mêmes, en barres |  |

Symboles : `shape.triangleup`, `triangledown`, `circle`, `cross`, `xcross`, `diamond`, `square`, `flag`, `arrowup`, `arrowdown`, `labelup`, `labeldown`. Places : `location.abovebar`, `belowbar`, `absolute`, `top`, `bottom`. Tailles : de `size.tiny` à `size.auto`.

Objets :

| Objet | Créer | Modifier |
| --- | --- | --- |
| Étiquette | `label.new(x, y, texte, style, color, textcolor, size)` | `label.set_text`, `set_xy`, `set_color`, `label.delete` |
| Ligne | `line.new(x1, y1, x2, y2, extend, color, style, width)` | `line.set_xy1`, `set_xy2`, `line.get_price(l, x)`, `line.delete` |
| Rectangle | `box.new(gauche, haut, droite, bas, border_color, bgcolor, text)` | `box.set_top`, `set_right`, `set_bgcolor`, `box.delete` |
| Remplissage entre deux lignes | `linefill.new(l1, l2, couleur)` | `linefill.set_color`, `linefill.delete` |
| Courbe par points | `polyline.new(points, curved, closed, line_color, fill_color, line_style, line_width)` | `polyline.delete` ; un point se fait avec `chart.point.from_index(index, prix)`, `chart.point.from_time(temps, prix)` ou `chart.point.now(prix)` |
| Tableau | `table.new(position, colonnes, lignes, bgcolor)` | `table.cell(t, col, ligne, texte, text_color, bgcolor)`, `table.cell_set_text`, `table.clear` |

- Par défaut, 500 étiquettes, 500 lignes et 500 rectangles au plus par script ; `indicator(..., max_labels_count = 100)` change la limite (1 à 500). Pour les courbes : 50 par défaut, `max_polylines_count` de 1 à 100. Passé la limite, le plus vieux dessin de cette sorte s'efface.
- Un dessin effacé avec `delete` ne se lit plus et ne se change plus : ses `get_*` donnent `na` et ses `set_*` ne font rien, comme dans Pine.
- Un tableau a 10 000 cases au plus. Le texte d'une case est noir par défaut : donne `text_color` sur fond sombre.
- Une méthode inconnue sur un objet ne plante pas : elle donne `na` sans message.
- Chaque tableau se déplace et se redimensionne à la souris ou au doigt ; la place est gardée pour l'indicateur.

Exemple : une étiquette sur chaque sommet confirmé et un petit tableau en haut à droite.

```
//@version=6
indicator("Sommets", overlay = true)
ph = ta.pivothigh(5, 5)
if not na(ph)
    label.new(bar_index - 5, ph, str.tostring(ph, "#.##"), style = label.style_label_down, color = color.red, textcolor = color.white)
var t = table.new(position.top_right, 2, 1, bgcolor = color.new(color.black, 30))
if barstate.islast
    table.cell(t, 0, 0, "RSI", text_color = color.white)
    table.cell(t, 1, 0, str.tostring(ta.rsi(close, 14), "#"), text_color = color.white)
```

## Les réglages

Chaque `input.*` du script devient une ligne de la fenêtre ⚙ de l'indicateur, sans rien dessiner à la main ; Argos garde les valeurs choisies.

```
//@version=6
indicator("Canal", overlay = true)
src   = input.source(close, "Source")
n     = input.int(20, "Longueur", minval = 2, maxval = 500, group = "Calcul")
mult  = input.float(2.0, "Écart", step = 0.1, group = "Calcul")
haut  = input.color(color.red, "Haut", inline = "c", group = "Couleurs")
bas   = input.color(color.green, "Bas", inline = "c", group = "Couleurs")
mode  = input.string("Simple", "Moyenne", options = ["Simple", "Exponentielle"])
m = mode == "Simple" ? ta.sma(src, n) : ta.ema(src, n)
s = ta.stdev(src, n)
plot(m + mult * s, "Haut", haut)
plot(m - mult * s, "Bas", bas)
```

| Réglage | Donne | À savoir |
| --- | --- | --- |
| `input.int`, `input.float` | Un nombre | `minval`, `maxval`, `step` bornent la saisie |
| `input.bool` | Une case à cocher |  |
| `input.string(..., options = [...])` | Une liste à choisir | `input.enum` fait pareil |
| `input.source` | Une série de prix | `close`, `open`, `high`, `low`, `hl2`, `hlc3`, `ohlc4`, `hlcc4`, `volume` |
| `input.color` | Une couleur |  |
| `input.price`, `input.time` | Un prix, une date |  |
| `input.timeframe` | Une unité de temps | Pour `request.security` |
| `input.text_area` | Un texte long |  |
| `input.session`, `input.symbol` | Du texte seulement | Pas encore de vraie session ni de vrai symbole |
| `input(valeur)` | Deviné d'après la valeur | `input(close)` devient une source |

Ordre des paramètres, comme dans Pine : valeur, titre, `minval`, `maxval`, `step`, `tooltip`, `inline`, `group`, `confirm`, `options`.

- `group` range les lignes sous un titre dans la fenêtre ⚙.
- `inline` met plusieurs réglages sur la même ligne.
- Deux réglages au même titre restent distincts : Argos les numérote en interne, rien à faire.

## Collections et texte

Listes (`array`), dictionnaires (`map`), matrices (`matrix`), texte (`str`) et couleurs (`color`) marchent comme dans Pine, et Argos ajoute les lambdas pour transformer une liste en une ligne.

```
//@version=6
indicator("Derniers volumes", overlay = false)
var vols = array.new_float()
vols.push(volume)                       // forme méthode, comme array.push(vols, volume)
if vols.size() > 20
    vols.shift()
total = array.reduce(vols, (acc, v) => acc + v, 0)   // lambda Argos
plot(total, "Volume des 20 dernières bougies")
```

| Famille | Fonctions | À savoir |
| --- | --- | --- |
| Créer une liste | `array.new<float>(taille, valeur)`, `array.new_float`, `array.from(1, 2, 3)` | Valeur par défaut `na` |
| Lire, écrire | `get`, `set`, `first`, `last`, `size`, `includes`, `indexof`, `lastindexof` | `get(-1)` lit depuis la fin |
| Ajouter, retirer | `push`, `unshift`, `pop`, `shift`, `insert`, `remove`, `clear`, `fill` | `fill` remplit toute la liste |
| Calculer | `sum`, `avg`, `max`, `min`, `range`, `median`, `stdev`, `variance` | `stdev` et `variance` biaisés |
| Ranger | `sort(order.ascending)`, `reverse`, `copy`, `slice`, `concat`, `join` | `slice` rend une copie |
| Lambdas (Argos) | `map`, `filter`, `reduce(liste, (acc, x) => acc + x, 0)`, `foreach`, `some`, `every`, `count`, `find` | Exemple : `array.map(a, (x) => x * 2)` |
| Dictionnaire | `map.new<string, float>()`, `put`, `get`, `contains`, `remove`, `size`, `keys`, `values`, `clear`, `copy` | Garde l'ordre d'ajout ; `put` rend l'ancienne valeur |
| Matrice | `matrix.new<float>(lignes, colonnes, valeur)`, `get`, `set`, `rows`, `columns`, `elements_count`, `row`, `col`, `add_row`, `add_col`, `remove_row`, `remove_col`, `swap_rows`, `swap_columns`, `fill`, `copy`, `transpose`, `reverse`, `sort`, `min`, `max`, `avg`, `is_square` | Lignes et colonnes comptées à partir de 0 |
| Texte | `str.tostring(x, "#.##")`, `str.tonumber`, `str.length`, `str.upper`, `str.lower`, `str.contains`, `str.startswith`, `str.endswith`, `str.pos`, `str.split`, `str.trim`, `str.repeat` | Formats `"#.##"`, `"0.00"`, `"percent"` ; `str(x)` = raccourci Argos |
| Remplacer | `str.replace(t, a, b)`, `str.replace_all` | `replace` change la première seulement |
| Mettre en forme | `str.format("{0} à {1,number,#.##}", a, b)` |  |
| Couleurs | `color.new(c, transparence)`, `color.rgb(r, g, b, t)`, `color.from_gradient`, `color.r`, `g`, `b`, `t` | Littéraux `#RRGGBB` ou `#RRGGBBAA` |

Couleurs nommées : `color.red` #FF5252, `green` #4CAF50, `lime` #00E676, `blue` #2196F3, `aqua` #00BCD4, `teal` #00897B, `yellow` #FFEB3B, `orange` #FF9800, `purple` #9C27B0, `fuchsia` #E040FB, `maroon` #880E4F, `navy` #311B92, `olive` #808000, `gray` #787B86, `silver` #B2B5BE, `white` #FFFFFF, `black` #363A45. Fond de la chart : `chart.bg_color` #131722, texte : `chart.fg_color` #D1D4DC.

Tailles : une liste ou une matrice a 100 000 éléments au plus (ou 5 par bougie de la chart, si c'est plus), un dictionnaire 50 000 paires, un texte 100 000 caractères. Au-delà, le script s'arrête avec un message qui le dit.

Pas encore là : les calculs de matrices (`matrix.mult`, `matrix.sum`, `matrix.inv` et les autres), `array.binary_search`, `array.percentile_*`, `array.covariance`, `array.mode`, `array.standardize`, `map.put_all`, `str.format_time`, `str.match`.

## Autres unités de temps

`request.security` lit une autre unité de temps du même symbole : une plus grande, par exemple le RSI du jour sur une chart en 1 h, ou une plus petite (la valeur à la dernière petite bougie de chaque bougie, comme TradingView).

```
//@version=6
indicator("RSI du jour", overlay = false)
rsiJour = request.security(syminfo.tickerid, "D", ta.rsi(close, 14))
plot(rsiJour, "RSI jour")
hline(70)
hline(30)
```

| Fonction | Donne | Règles |
| --- | --- | --- |
| `request.security(symbole, unité, expression)` | La valeur de l'expression sur l'autre unité | Même symbole seulement ; unité plus grande, égale ou plus petite (plus petite : la valeur à la dernière petite bougie de chaque bougie, avec les petites bougies que l'app charge, comme pour `request.security_lower_tf`) ; `gaps_off` et `lookahead_off` ; le script refait un passage sur ces bougies-là ; un dessin fait dans l'expression (`label.new`…) vaut `na` sur la chart |
| `request.security_lower_tf(symbole, unité, expression)` | Une liste par bougie avec les valeurs des petites bougies dedans | 10 000 petites bougies au plus, puis les 200 plus récentes toutes les 60 s |

Exemple : volume acheteur et vendeur de chaque bougie de 1 h vu à travers ses bougies de 1 min.

```
//@version=6
indicator("Volume 1 min", overlay = false)
vols = request.security_lower_tf(syminfo.tickerid, "1", close > open ? volume : -volume)
net = array.sum(vols)
plot(net, "Net", net >= 0 ? color.green : color.red, style = plot.style_columns)
```

Sur Hyperliquid, les petites bougies ne remontent pas loin (par exemple environ 86 bougies de 1 h en 1 min) : avant, la liste est vide et le script doit prévoir un calcul de repli. Le terminal le signale.

Refusé avec un message clair : un autre symbole, `gaps_on`, et les autres `request.*`.

## Stratégies (compte démo)

Un script `strategy(...)` passe des ordres sur le compte démo d'Argos seulement, jamais chez un courtier : on voit ses entrées, ses sorties et son résultat sans risque.

```
//@version=6
strategy("Croisement EMA", overlay = true, initial_capital = 10000,
     default_qty_type = strategy.percent_of_equity, default_qty_value = 10,
     commission_type = strategy.commission.percent, commission_value = 0.05)
rapide = ta.ema(close, 9)
lente  = ta.ema(close, 21)
if ta.crossover(rapide, lente)
    strategy.entry("Achat", strategy.long)
if ta.crossunder(rapide, lente)
    strategy.close("Achat")
strategy.exit("Sortie", "Achat", profit = 400, loss = 200)
plot(rapide, "Rapide", color.orange)
plot(lente, "Lente", color.blue)
```

| Famille | Noms | À savoir |
| --- | --- | --- |
| Ordres | `strategy.entry`, `strategy.order`, `strategy.exit`, `strategy.close`, `strategy.close_all`, `strategy.cancel`, `strategy.cancel_all` | `exit` accepte `profit`, `limit`, `loss`, `stop`, `trail_*` ; `when = false` saute l'ordre |
| Sens et quantité | `strategy.long`, `strategy.short`, `strategy.fixed`, `strategy.cash`, `strategy.percent_of_equity` | Quantité par défaut dans `strategy(...)` |
| Position | `strategy.position_size`, `position_avg_price`, `position_entry_name`, `openprofit` | Taille positive = achat, négative = vente |
| Résultats | `strategy.netprofit`, `equity`, `initial_capital`, `grossprofit`, `grossloss`, `max_drawdown` |  |
| Comptes | `strategy.opentrades`, `closedtrades`, `wintrades`, `losstrades`, `eventrades` |  |

Paramètres de `strategy(...)` : `pyramiding`, `default_qty_type`, `default_qty_value`, `initial_capital`, `slippage`, `commission_type`, `commission_value`, `process_orders_on_close`. Ils apparaissent dans la fenêtre ⚙ sous « Propriétés de la stratégie ».

- Commission : seulement en pourcentage pour l'instant.
- Les variables `strategy.*` ne marchent que dans un script `strategy(...)` ; dans un `indicator(...)`, l'éditeur le dit.
- Ce chapitre servira de base au futur module « Bots et stratégies », toujours sur compte démo.

## Alertes et journal

`alertcondition` crée un signal nommé qu'on choisit ensuite dans la fenêtre des alertes ; `alert()` et `log.*` écrivent seulement dans le terminal de l'éditeur.

```
//@version=6
indicator("Alerte RSI", overlay = false)
r = ta.rsi(close, 14)
plot(r, "RSI")
alertcondition(ta.crossover(r, 70), "RSI au-dessus de 70", "Le RSI vient de passer 70")
if barstate.islast
    log.info("RSI actuel : " + str.tostring(r, "#.#"))
```

| Fonction | Fait | Où on le voit |
| --- | --- | --- |
| `alertcondition(cond, titre, message)` | Déclare un signal | Fenêtre des alertes : choisir l'indicateur, puis le titre |
| `alert(message)` | Écrit un message | Terminal de l'éditeur |
| `log.info`, `log.warning`, `log.error` | Écrit une ligne de suivi | Terminal de l'éditeur, les 5 dernières |

Pour déboguer, `log.info` sur `barstate.islast` montre une valeur sans encombrer le terminal.

## Recettes

Quatre scripts complets à coller tels quels dans l'éditeur ; la stratégie d'exemple est dans la partie Stratégies. Pas encore passés dans l'éditeur : ils seront essayés au prochain tour de compilation.

**Supertrend en deux couleurs, avec flèches au changement**

```
//@version=6
indicator("Supertrend", overlay = true)
facteur = input.float(3.0, "Facteur", step = 0.1)
periode = input.int(10, "Période ATR", minval = 1)
[st, dir] = ta.supertrend(facteur, periode)
plot(dir < 0 ? st : na, "Haussier", color.green, style = plot.style_linebr)
plot(dir > 0 ? st : na, "Baissier", color.red, style = plot.style_linebr)
plotshape(ta.change(dir) < 0, "Achat", shape.triangleup, location.belowbar, color.green)
plotshape(ta.change(dir) > 0, "Vente", shape.triangledown, location.abovebar, color.red)
```

**Plus haut et plus bas de la veille**

```
//@version=6
indicator("Veille", overlay = true)
hautVeille = request.security(syminfo.tickerid, "D", high[1])
basVeille  = request.security(syminfo.tickerid, "D", low[1])
plot(hautVeille, "Haut veille", color.red, style = plot.style_stepline)
plot(basVeille, "Bas veille", color.green, style = plot.style_stepline)
```

**RSI sur trois unités dans un tableau** (chart en 1 h ou moins)

```
//@version=6
indicator("RSI multi-UT", overlay = true)
r1 = request.security(syminfo.tickerid, "60", ta.rsi(close, 14))
r4 = request.security(syminfo.tickerid, "240", ta.rsi(close, 14))
rD = request.security(syminfo.tickerid, "D", ta.rsi(close, 14))
coul(r) => r > 70 ? color.red : r < 30 ? color.green : color.gray
var t = table.new(position.top_right, 2, 3, bgcolor = color.new(color.black, 30))
if barstate.islast
    table.cell(t, 0, 0, "1 h", text_color = color.white)
    table.cell(t, 1, 0, str.tostring(r1, "#"), text_color = coul(r1))
    table.cell(t, 0, 1, "4 h", text_color = color.white)
    table.cell(t, 1, 1, str.tostring(r4, "#"), text_color = coul(r4))
    table.cell(t, 0, 2, "Jour", text_color = color.white)
    table.cell(t, 1, 2, str.tostring(rD, "#"), text_color = coul(rD))
```

**Bougies à gros volume**

```
//@version=6
indicator("Gros volume", overlay = true)
mult = input.float(2.0, "Fois la moyenne", step = 0.5)
moy = ta.sma(volume, 20)
gros = volume > mult * moy
barcolor(gros ? (close > open ? color.lime : color.fuchsia) : na)
plotchar(gros, "Gros volume", "●", location.abovebar, color.yellow, size = size.tiny)
```

## Importer du Pine et du wrun

On colle le script tel quel via « Importer du Pine Script » : Argos reconnaît d'où il vient (pour Pine, à la ligne `//@version=`) et le traduit tout seul avant de le lancer.

| Origine | Ce qu'Argos fait |
| --- | --- |
| Pine v6 | Lu tel quel ; `input(close)` devient `input.source` |
| Pine v5 | Version passée en v6 ; lignes `import` mises en commentaire ; `and` / `or` calculent les deux côtés, comme en v5 |
| Pine v4 | `sma(...)` devient `ta.sma(...)`, `abs(...)` devient `math.abs(...)` ; `study` devient `indicator` ; `security` devient `request.security` ; `tostring` / `tonumber` vont dans `str.*` ; `iff(a, b, c)` devient `a ? b : c` ; `input(type = ...)` devient `input.*` ; `transp` retiré ; `tr`, `obv`, `vwap`, `accdist` vont dans `ta.*` |
| Argos Script v1 | `signal()`, `shape()`, `fill(a, b, c1, c2)`, `prev(x, n)` traduits |
| wrun | `param` devient `input` ; `output` devient `plot` ; le corps de `onBar` devient le script ; `new Ema(n).update(x)` devient `ta.ema(x, n)` ; `bar.*` devient les séries de prix ; `Math.*` devient `math.*` ; `alert(quand)` devient `alertcondition` ; `render.*` devient `plotshape`, `bgcolor` ou `barcolor` |

Pour un Pine v4, un avertissement rappelle que `transp` est retiré : la transparence passe par `color.new(couleur, transparence)`.

Si une fonction manque, le message le dit avec son nom, par exemple « ta.alma : pas encore dans Argos Script » ; la liste complète est dans la partie Limites.

## Erreurs courantes

Le terminal écrit « ❌ Ligne N : message » ; voici les messages qu'on croise le plus, mot pour mot, et quoi faire.

| Message | Cause | Quoi faire |
| --- | --- | --- |
| « x » n'existe pas (déclare-la d'abord avec « x = … ») | Variable changée avec `:=` avant d'exister | Écrire `x = …` une première fois |
| « x » n'existe pas (ou pas encore dans Argos Script) | Nom mal écrit, ou fonction absente | Vérifier l'orthographe, puis la partie Limites |
| ta.alma : pas encore dans Argos Script | Fonction pas encore faite | La remplacer, ou demander qu'on l'ajoute |
| cette fonction donne plusieurs valeurs : [a, b] = … | `ta.macd`, `ta.bb`, `ta.dmi` écrits sans crochets | `[m, s, h] = ta.macd(close, 12, 26, 9)` |
| indentation incohérente | Mélange d'espaces dans un bloc | Blocs à 4 espaces exactement |
| bloc indenté attendu (4 espaces) | Rien sous un `if`, un `for` ou une fonction | Indenter la ligne suivante de 4 espaces |
| texte pas fermé (guillemet manquant) | Un `"` oublié | Fermer le texte |
| « ) » attendu, trouvé … | Parenthèse ou virgule manquante | Regarder juste avant l'endroit indiqué |
| couleur invalide | `#` suivi d'autre chose que 6 ou 8 chiffres hexadécimaux | `#FF5252` ou `#FF525280` |
| fill() : deux plot ou hline | `fill` entre autre chose | Garder les résultats : `p1 = plot(...)` |
| strategy.x : seulement dans un script strategy(...) | Variable de stratégie dans un indicateur | Remplacer `indicator(` par `strategy(` |
| array index hors du tableau | Lecture au-delà de la taille | Tester `array.size` avant |
| array.map : il faut une lambda, ex. (x) => x * 2 | Fonction passée au mauvais format | Écrire `(x) => …` |
| table.new : trop de cases (10 000 au plus) | Tableau trop grand | Moins de lignes ou de colonnes |
| trop d'opérations sur une bougie | Boucle trop longue (2 000 000 d'opérations par bougie) | Réduire la boucle, ou garder le résultat dans un `var` |
| récursion | Une fonction s'appelle elle-même trop profond (64) | Réécrire en boucle |
| le script garde trop de mémoire : plus de … Mo | Trop de listes, de textes, d'objets, de dessins ou d'historiques gardés d'une bougie à l'autre | Raccourcir ou vider les listes (`array.shift`, `array.clear`), effacer les dessins qui ne servent plus |
| trop de niveaux l'un dans l'autre (parenthèses, appels, blocs ou ? :) : moins de 100 | 100 parenthèses, appels ou `? :` l'un dans l'autre, ou 50 blocs (un bloc dans un bloc compte pour deux ; dans un vieux Pine, un `iff` compte pour deux) | Couper le calcul en morceaux, une variable par morceau |
| expression trop longue ou trop imbriquée : moins de 250 termes à la suite, moins quand les premiers sont eux-mêmes des calculs, des appels ou des [n] | 250 termes ou plus dans un seul calcul (`a + b + c + …`), même écrit sur plusieurs lignes ; 249 quand les premiers termes sont des calculs, des appels, des champs ou des `[n]` (`2 * close`, `nz(close)`, `syminfo.mintick`, `close[1]`, `close > 0` dans une suite de `and`) | La couper en morceaux, une variable par morceau |
| script trop long : 1 Mo au plus (il dépasse à cette ligne) | Un script de plus d'un million d'octets (les plus gros scripts connus font 50 Ko) | Le raccourcir |
| trop de niveaux l'un dans l'autre dans ce calcul : 250 au plus | Des fonctions qui s'appellent l'une l'autre depuis beaucoup de blocs | Simplifier, ou réécrire en boucle |
| array.push : 100 000 éléments au plus | Liste ou matrice trop grande | Garder moins d'éléments (`array.shift`) |
| map.put : 50 000 paires au plus | Dictionnaire trop grand | Retirer les vieilles clés (`map.remove`) |
| str.repeat : 100 000 caractères au plus dans un texte | Texte trop long | Raccourcir le texte |
| request.security : … | Autre symbole ou `gaps_on` | Voir la partie Autres unités de temps |

Avertissements (le script tourne quand même) :

- « La fonction « ta.rsi » doit être appelée à chaque bougie … sortez cet appel de cette condition » : voir la partie Les calculs.
- « transp » retiré (v4) : utiliser `color.new`.

Messages de l'éditeur :

- « ❌ Ce script ne trace rien … » : ajouter au moins un `plot`, `plotshape`, `bgcolor`, `label` ou `table`.
- « ⏳ Pas encore de bougies … » : attendre le chargement de la chart, puis ▶ à nouveau.
- « ⚠ Rien à tracer … » : le script tourne mais toutes ses valeurs sont `na` (longueur plus grande que l'historique, condition jamais vraie).

## Limites et différences avec Pine

Le script tourne sur l'appareil, pas sur un serveur ; ces limites protègent la chart d'un script qui boucle sans fin ou qui garde trop de choses.

| Limite | Valeur |
| --- | --- |
| Opérations par bougie | 2 000 000 |
| Profondeur de récursion | 64 |
| Mémoire gardée par un script | 256 Mo, plus 1 Ko par bougie de la chart |
| Parenthèses, appels ou `? :` l'un dans l'autre | 99 (un `iff` d'un vieux Pine compte pour deux : 49) |
| Blocs l'un dans l'autre (`if`, `for`, `while`, `switch`) | 49 |
| Termes à la suite dans un calcul | 249 noms ou nombres ; 248 quand les premiers sont des calculs, des appels, des champs ou des `[n]` (`2 * close`, `nz(close)`, `close[1]`) ; un peu moins s'ils ont eux-mêmes plusieurs niveaux (247 pour `math.abs(-1)`) |
| Taille d'un script | 1 Mo |
| Niveaux d'un calcul (blocs, fonctions qui s'appellent) | 250 |
| Éléments d'une liste ou d'une matrice | 100 000 (ou 5 par bougie de la chart, si c'est plus) |
| Paires d'un dictionnaire | 50 000 |
| Caractères d'un texte | 100 000 |
| Étiquettes, lignes, rectangles par script | 500 de chaque sorte au plus (réglable de 1 à 500) |
| Courbes (`polyline`) | 50 par défaut (réglable de 1 à 100) |
| Remplissages entre lignes | 500 |
| Cases d'un tableau | 10 000 |
| `request.security_lower_tf` | 10 000 petites bougies par unité, puis les 200 plus récentes toutes les 60 s |
| Journal | Les 5 dernières lignes dans le terminal |

Ce qui change par rapport à Pine :

| Sujet | Dans Argos Script |
| --- | --- |
| Fuseau horaire | Tout en UTC ; le fuseau donné à `timestamp` est ignoré |
| Types | `int` et `float` sont le même nombre ; les types écrits sont ignorés |
| `varip` | Fait comme `var` |
| `barstate.isnew` | Toujours vrai |
| `syminfo` | Devise USD, type crypto, `mintick` déduit des dernières clôtures |
| `request.security` | Même symbole et unité plus grande seulement |
| Paramètres ignorés sans message | `max_bars_back`, `trackprice`, `histbase`, `yloc`, largeurs de cases des tableaux, entre autres |
| En plus de Pine | Lambdas, `array.map` / `filter` / `reduce` / `foreach` / `some` / `every` / `count` / `find`, `argos.real_delta`, `str(x)`, import wrun et Argos v1 |

Pas encore là : `runtime.error`, `ticker.*`, `session.*`, `time()` avec une séance, `str.format_time`, `str.match`, `library`, `import` / `export`, `enum`, et les fonctions listées dans chaque partie.
