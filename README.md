# Ship Routing

Die finale Abgabe wurde bei mir etwas chaotischer, daher hier ein bissschen Beschreibung:

Ich habe die Abgabe aufgeteilt, da ich mein Path Finding ode danach veröffentlichen möchte. Meine Abgabe besteht aus drei Teilen:
1) diesem Crate. Hier wird der Server für das Routing gestartet
2) osm_convert. Dies generiert die .gr und .co Dateien.
3) faster_paths. Dies arbeiter auf den .gr und .co Dateien

Ein kleines How-To:
1) neues Dir erstellen
2) git clone git@github.com:christianstaib/faster_paths.github
    make dirs
    make download
    make convert_osm
    make generate_network
3) git clone git@github.com:christianstaib/osm_converter.github
    make create_ch
    make create_hl
4) in diesem dir make Server

Das front end bleibt gleich zur ersten Abgabe
