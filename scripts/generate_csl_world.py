#!/usr/bin/env python3
"""
Generate a Chinese Super League (CSL) world database for OpenFoot Manager.
"""
import json, uuid, random

random.seed(42)

# Track used names to avoid duplicates
_used_chinese_names = set()
_used_foreign_names = set()

# ── 24 teams ──
TEAMS = [
    ("成都蓉城", "成都", ["#1a3c6e", "#e31b23"]),
    ("大连英博", "大连", ["#1a5c9e", "#ffffff"]),
    ("重庆铜梁龙", "重庆", ["#c8102e", "#f5a623"]),
    ("云南玉昆", "昆明", ["#702c91", "#ffd700"]),
    ("山东泰山", "济南", ["#de0f17", "#1a3c6e"]),
    ("河南", "郑州", ["#e31b23", "#ffd700"]),
    ("浙江FC", "杭州", ["#00a650", "#1a3c6e"]),
    ("青岛西海岸", "青岛", ["#1d76c2", "#f58220"]),
    ("上海申花", "上海", ["#004098", "#ffffff"]),
    ("上海海港", "上海", ["#e31b23", "#f5a623"]),
    ("北京国安", "北京", ["#006633", "#f5a623"]),
    ("深圳新鹏城", "深圳", ["#e31b23", "#1a3c6e"]),
    ("辽宁铁人", "沈阳", ["#e31b23", "#ffd700"]),
    ("青岛海牛", "青岛", ["#1d76c2", "#ffffff"]),
    ("武汉三镇", "武汉", ["#e31b23", "#f5a623"]),
    ("天津津门虎", "天津", ["#1a3c6e", "#ffffff"]),
    ("延边龙鼎", "延吉", ["#e31b23", "#ffd700"]),
    ("陕西联合", "西安", ["#1a3c6e", "#e31b23"]),
    ("广西恒宸", "南宁", ["#e31b23", "#ffd700"]),
    ("苏州东吴", "苏州", ["#1a3c6e", "#ffffff"]),
    ("上海海港B队", "上海", ["#e31b23", "#f5a623"]),
    ("深圳二零二八", "深圳", ["#1d76c2", "#ffffff"]),
    ("湖北青年星", "武汉", ["#e31b23", "#1a3c6e"]),
    ("喀什", "喀什", ["#702c91", "#ffd700"]),
]
# ── Chinese surname / given-name pools ──
HAN_SURNAMES = "王 李 张 刘 陈 杨 赵 黄 周 吴 徐 孙 胡 朱 高 林 何 郭 马 罗 梁 宋 郑 谢 韩 唐 冯 董 肖 田 曹 袁 邓 许 傅 沈 曾 彭 吕 苏 卢 蒋 蔡 贾 丁 魏 薛 叶 阎 余 潘 杜 戴 夏 钟 汪 田 任 姜 范 方 石 姚 谭 廖 邹 熊 金 陆 郝".split()
HAN_GIVEN = ["志强","伟杰","明辉","浩宇","俊杰","建平","志明","永强","建国","文杰","海涛","卫东","建华","志伟","嘉诚","瑞华","晓明","洪波","泽宇","天佑","宇轩","子涵","梓豪","雨泽","俊豪","博文","鹏飞","伟国","振华","家豪","明杰","志鹏","嘉豪","瑞祥","庆丰","德明","光华","世杰","宏亮","国栋","海峰","文斌","永春","金明","秀峰","玉林","宝山","福生","少华","广平","庆华","建峰","惠明","瑞林","洪亮","志坚","伟民","永康","胜军","长春","新宇","云龙","大军","克强","宗伟","永林","金水","德福","贵生","玉山","富贵","万全","文涛","建新","海波","瑞鑫","铭泽","景辉","卫平"]

# ── Ethnic minority name pools (5% of players) ──
MINORITY_SURNAMES = ["买买提","阿卜杜","古丽","伊力","巴图","乌兰","格日乐","阿斯哈","库尔班","赛买提","阿不力孜","吐尔逊","苏莱曼","马木提","热合曼","玉素甫","艾麦提","吾斯曼","尼亚孜","亚森","多里坤","阿依肯","胡万","阿德尔","巴特尔","德吉","扎西","央金","卓玛","次仁","桑吉","贡布","纳森","嘎达","其其格","孟和","敖敦"]
MINORITY_GIVEN = ["江","明","亮","峰","军","平","华","强","刚","涛","磊","伟","东","建","文","海","浩","鹏","飞","斌","泉","成","云","山","龙","峰","峰"]

def random_chinese_name(is_minority=False):
    global _used_chinese_names
    for _ in range(200):  # retry limit
        if is_minority:
            s = random.choice(MINORITY_SURNAMES)
            g = random.choice(MINORITY_GIVEN)
        else:
            s = random.choice(HAN_SURNAMES)
            g = random.choice(HAN_GIVEN)
        name = s + g
        if name not in _used_chinese_names:
            _used_chinese_names.add(name)
            return name, name  # full_name, match_name
    # Fallback: add a number suffix
    name = s + g + str(random.randint(1, 99))
    _used_chinese_names.add(name)
    return name, name

def random_id():
    return str(uuid.uuid4())

def gen_attributes(base, variance=8, is_foreign=False, is_gk=False):
    """Generate player attributes around a base overall rating."""
    if is_foreign:
        base = int(base * 1.25)  # foreign players 25% stronger
    r = lambda: max(1, min(99, base + random.randint(-variance, variance)))
    attrs = {
        "pace": r(), "stamina": r(), "strength": r(), "agility": r(),
        "passing": r(), "shooting": r(), "tackling": r(), "dribbling": r(),
        "defending": r(), "positioning": r(), "vision": r(), "decisions": r(),
        "composure": r(), "aggression": r(), "teamwork": r(), "leadership": r(),
        "handling": 30, "reflexes": 30, "aerial": r()
    }
    if is_gk:
        attrs.update({"handling": base + random.randint(-5,5), "reflexes": base + random.randint(-5,5), "aerial": base + random.randint(-5,5)})
    return attrs

POSITIONS = ["Goalkeeper","Defender","Midfielder","Forward"]
POS_WEIGHTS = [0.12, 0.33, 0.33, 0.22]

def pick_position():
    return random.choices(POSITIONS, weights=POS_WEIGHTS, k=1)[0]

def generate_player(team_id, is_foreign=False, is_minority=False, base_ovr=None, is_gk=False):
    pid = random_id()
    full_name, match_name = random_chinese_name(is_minority)
    if is_foreign:
        # Foreign players get English names
        foreign_first = ["James","Marcus","Oliver","Lucas","Gabriel","Diego","Rafael","Bruno","Joao","Miguel","Luis","Carlos","Andre","Paulo","Sergio","Eduardo","Felipe","Marco","Giovanni","Lorenzo","Alessandro","Francesco","Matteo","Lukas","Felix","Maximilian","Yannick","Thibaut","Arnaud","Baptiste"]
        foreign_last = ["Silva","Santos","Rodriguez","Martinez","Garcia","Lopez","Fernandez","Gonzalez","Perez","Sanchez","Ramirez","Torres","Rivera","Morales","Ortiz","Castro","Reyes","Mendoza","Delgado","Vargas","Cruz","Ramos","Diaz","Muller","Schmidt","Wagner","Becker","Hoffmann","Fischer","Weber"]
        for _ in range(200):
            fn = random.choice(foreign_first)
            ln = random.choice(foreign_last)
            full_name = f"{fn} {ln}"
            if full_name not in _used_foreign_names:
                _used_foreign_names.add(full_name)
                break
        match_name = full_name
        nationality = random.choice(["Brazil","Argentina","Portugal","Spain","France","Netherlands","Germany","Italy","England","Belgium","Croatia","Serbia","Nigeria","Senegal","Ivory Coast","Ghana","Cameroon","Mali","DR Congo","Angola","Colombia","Uruguay","Chile","Paraguay","Ecuador","Peru","Venezuela","Costa Rica","Mexico","USA","Canada","Australia","South Korea","Japan","Saudi Arabia","Qatar","Iran","Iraq","Egypt","Morocco","Algeria","Tunisia","South Africa","Zambia","Zimbabwe","Mozambique","Guinea","Burkina Faso","Cape Verde","Guinea-Bissau","Sierra Leone","Liberia","Togo","Benin","Niger","Chad","Central African Republic","Congo","Gabon","Equatorial Guinea","Rwanda","Burundi","Uganda","Kenya","Tanzania","Ethiopia","Eritrea","Djibouti","Somalia","Sudan","South Sudan","Mauritania","Senegal","Gambia","Mali","Burkina Faso","Benin","Togo","Ghana","Ivory Coast","Liberia","Sierra Leone","Guinea","Guinea-Bissau","Nigeria","Cameroon","Gabon","Angola"])
    else:
        nationality = "China"
    
    age = random.randint(18, 35)
    birth_year = 2026 - age
    dob = f"{birth_year}-{random.randint(1,12):02d}-{random.randint(1,28):02d}"
    position = "Goalkeeper" if is_gk else pick_position()
    
    if base_ovr is None:
        base_ovr = random.randint(45, 75) if nationality == "China" else random.randint(65, 85)
    
    player = {
        "id": pid,
        "match_name": match_name,
        "full_name": full_name,
        "date_of_birth": dob,
        "nationality": nationality,
        "position": position,
        "natural_position": position,
        "attributes": gen_attributes(base_ovr, is_foreign=(nationality!="China"), is_gk=is_gk),
        "condition": random.randint(75, 100),
        "morale": random.randint(50, 100),
        "fitness": random.randint(60, 100),
        "injury": None,
        "team_id": team_id,
        "retired": False,
        "squad_role": "Senior",
        "traits": [],
        "ovr": 0,
        "potential": 0,
        "contract_end": f"{birth_year + random.randint(1,4)}-06-30",
        "wage": random.randint(2000, 50000) if nationality!="China" else random.randint(1000, 15000),
        "market_value": random.randint(50000, 5000000) if nationality!="China" else random.randint(10000, 800000),
        "stats": {"appearances":0,"goals":0,"assists":0,"clean_sheets":0,"avg_rating":6.5,"minutes_played":0,"yellow_cards":0,"red_cards":0,"shots":0,"shots_on_target":0,"passes_completed":0,"passes_attempted":0,"tackles_won":0,"interceptions":0,"fouls_committed":0,"player_of_match":0,"wins":0,"losses":0,"draws":0},
        "career": [],
        "transfer_listed": False,
        "loan_listed": False,
        "transfer_offers": [],
        "morale_core": {"manager_trust":50,"unresolved_issue":None,"recent_treatment":None,"pending_promise":None,"training_effort":None,"last_warning":"","unhappiness_counter":0,"morale_decay_counter":0}
    }
    return player

def generate_players_for_team(team_id, team_name, player_count=25):
    """Generate a squad for a team. Up to 5 foreign players (except 浙江FC)."""
    players = []
    max_foreign = 5
    is_zhejiang = team_name == "浙江FC"
    
    # Determine team strength
    if is_zhejiang:
        base_ovr_range = (40, 65)  # mid-lower tier
    elif team_name in ["上海海港","上海申花","北京国安","山东泰山"]:
        base_ovr_range = (55, 80)  # strong teams
    elif team_name in ["喀什","湖北青年星","深圳二零二八","上海海港B队","苏州东吴","广西恒宸"]:
        base_ovr_range = (40, 65)  # weaker teams
    else:
        base_ovr_range = (45, 72)  # mid table
    
    # Generate goalkeepers (2-3)
    gk_count = random.randint(2, 3)
    for _ in range(gk_count):
        b = random.randint(base_ovr_range[0], base_ovr_range[1])
        p = generate_player(team_id, is_foreign=False, is_gk=True, base_ovr=b)
        players.append(p)
    
    # Generate foreign players (up to max_foreign, except 浙江FC)
    foreign_count = 0
    if not is_zhejiang:
        foreign_count = random.randint(3, max_foreign)
        for _ in range(foreign_count):
            b = random.randint(70, 90)  # foreign players are strong
            p = generate_player(team_id, is_foreign=True, base_ovr=b)
            players.append(p)
    
    # Generate Chinese outfield players to fill up to player_count
    remaining = player_count - gk_count - foreign_count
    for i in range(remaining):
        is_minority = random.random() < 0.05  # 5% minority names
        b = random.randint(base_ovr_range[0], base_ovr_range[1])
        p = generate_player(team_id, is_foreign=False, is_minority=is_minority, base_ovr=b)
        players.append(p)
    
    return players

def generate_league(teams):
    """Generate a double round-robin league with 24 teams (each plays each other twice)."""
    # Generate 46 matchdays: team A vs B home, then B vs A away
    team_ids = [t["id"] for t in teams]
    n = len(team_ids)
    fixtures = []
    fid = 0

    def round_robin_round(ids, is_reverse_leg):
        """Generate one full round of n-1 matchdays using the circle method."""
        nonlocal fid
        round_fixtures = []
        mid = n // 2
        cur = ids[:]
        for rd in range(n - 1):
            md = rd if not is_reverse_leg else rd + n - 1
            for i in range(mid):
                home = cur[i]
                away = cur[n - 1 - i]
                # Alternate home/away in reverse leg
                if (rd + is_reverse_leg) % 2 == 1:
                    home, away = away, home
                fdate = f"2026-08-{1 + md:02d}"
                round_fixtures.append({
                    "id": f"csl_f{fid}",
                    "home_team_id": home,
                    "away_team_id": away,
                    "date": fdate,
                    "status": "Scheduled",
                    "competition": "League",
                    "matchday": md + 1,
                    "result": None,
                    "counts_for_league_standings": True,
                    "generates_match_report_news": True
                })
                fid += 1
            # Rotate: keep index 0 fixed, rotate rest clockwise
            cur = [cur[0]] + [cur[-1]] + cur[1:-1]
        return round_fixtures

    fixtures = round_robin_round(team_ids, False) + round_robin_round(team_ids, True)

    league = {
        "id": "csl_2026",
        "name": "Chinese Super League",
        "season": 2026,
        "fixtures": fixtures,
        "standings": [],
        "transfer_log": [],
        "transfer_rumours": []
    }
    
    for team in teams:
        league["standings"].append({
            "team_id": team["id"],
            "played": 0, "won": 0, "drawn": 0, "lost": 0,
            "goals_for": 0, "goals_against": 0, "points": 0,
            "form": []
        })
    
    return league

# ── Main generation ──
all_teams = []
all_players = []

for name, city, colors in TEAMS:
    tid = random_id()
    team = {
        "id": tid,
        "name": name,
        "short_name": name[:3],
        "country": "China",
        "football_nation": "CN",
        "city": city,
        "stadium_name": f"{name}体育场",
        "stadium_capacity": random.randint(15000, 55000),
        "finance": random.randint(2000000, 50000000),
        "manager_id": None,
        "reputation": random.randint(300, 800),
        "wage_budget": random.randint(500000, 3000000),
        "transfer_budget": random.randint(500000, 10000000),
        "season_income": 0,
        "season_expenses": 0,
        "financial_ledger": [],
        "facilities": {"training": random.randint(5,18), "medical": random.randint(5,18), "scouting": random.randint(5,18)},
        "formation": random.choice(["4-3-3","4-4-2","3-5-2","4-2-3-1","5-3-2","4-1-4-1"]),
        "play_style": random.choice(["Balanced","Attacking","Defensive","Possession","Counter","HighPress"]),
        "training_focus": "Technical",
        "training_intensity": "Medium",
        "training_schedule": "Balanced",
        "founded_year": random.randint(1950, 2010),
        "colors": {"primary": colors[0], "secondary": colors[1]},
        "training_groups": [],
        "starting_xi_ids": [],
        "match_roles": {"captain":None,"vice_captain":None,"penalty_taker":None,"free_kick_taker":None,"corner_taker":None},
        "form": [],
        "history": []
    }
    all_teams.append(team)
    team_players = generate_players_for_team(tid, name)
    all_players.extend(team_players)

# Generate league
league = generate_league(all_teams)

# Build WorldData
world = {
    "name": "Chinese Super League 2026",
    "description": "24-team Chinese Super League with realistic Chinese player names and up to 5 foreign players per team",
    "teams": all_teams,
    "players": all_players,
    "staff": [],
    "managers": [],
    "league": league,
    "news": [],
    "stats": {"player_matches":[],"team_matches":[]},
    "world_history": {"archives":[],"rivalries":[],"hall_of_fame":[]},
    "metadata": {"kind": "rosterBaseline", "base_year": 2026}
}

output_path = "scripts/csl_world_2026.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(world, f, ensure_ascii=False, indent=2)

team_counts = {}
for p in all_players:
    nat = p["nationality"]
    team_counts[nat] = team_counts.get(nat, 0) + 1

print(f"✅ CSL world generated: {output_path}")
print(f"   Teams: {len(all_teams)}")
print(f"   Players: {len(all_players)}")
print(f"   Chinese players: {team_counts.get('China', 0)}")
print(f"   Foreign players: {sum(v for k,v in team_counts.items() if k != 'China')}")
print(f"   Nationalities: {len(team_counts)}")
