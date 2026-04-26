#!/usr/bin/env python3
"""
Generate Chinese Super League database with balanced teams.
- Each team: max 5 foreign players (except 浙江FC which has 0 foreigners)
- Foreign players are higher quality than Chinese players
- Each team: max 3 minority ethnic players
- Team differences are reduced for balanced gameplay
- Contract dates are spread throughout the season
"""

import json
import random

random.seed(42)

# Chinese name pools - realistic variety
# Two-character first names (most common)
CHINESE_FIRST_2CHAR = [
    "伟", "强", "磊", "浩", "杰", "鹏", "飞", "超", "龙", "凯",
    "文", "勇", "波", "峰", "华", "刚", "洋", "军", "涛", "明",
    "东", "雷", "宇", "晨", "辉", "松", "健", "斌", "威", "林",
    "海", "川", "博", "然", "程", "思", "雨", "泽", "睿", "鑫",
    "俊", "瑞", "志", "强", "勇", "超", "鹏", "飞", "龙", "涛"
]

# Three-character first names (common)
CHINESE_FIRST_3CHAR = [
    "浩然", "子轩", "博文", "俊杰", "明轩", "志强", "宇轩", "浩宇",
    "子涵", "梓轩", "俊杰", "浩然", "思远", "宇航", "子晨", "嘉豪",
    "睿轩", "天宇", "子龙", "俊杰", "晨轩", "浩然", "文博", "宇航",
    "子琪", "梓豪", "浩天", "明轩", "志远", "子墨", "一凡", "浩然",
    "欣怡", "子萱", "思雨", "雨萱", "欣悦", "子涵", "梦瑶", "思琪"
]


# Single-character last names (most common)
CHINESE_LAST_1CHAR = [
    "王", "李", "张", "刘", "陈", "杨", "黄", "赵", "周", "吴",
    "徐", "孙", "马", "朱", "胡", "郭", "何", "高", "林", "罗",
    "郑", "梁", "谢", "宋", "唐", "许", "韩", "冯", "邓", "曹",
    "彭", "曾", "肖", "田", "董", "袁", "潘", "于", "蒋", "蔡",
    "余", "杜", "叶", "程", "苏", "魏", "吕", "丁", "任", "沈"
]


# Two-character last names (rare but realistic)
CHINESE_LAST_2CHAR = [
    "欧阳", "司马", "上官", "诸葛", "慕容", "令狐", "公孙", "轩辕",
    "夏侯", "呼延", "皇甫", "尉迟", "万俟", "澹台", "公冶", "宰父",
    "谷梁", "拓跋", "夹谷", "宰衡", "辛阕", "且卯", "梁丘", "左丘"
]

# Minority ethnic player names
MINORITY_FIRST_NAMES = [
    "阿不都", "艾克", "买提", "木热", "阿里", "穆塔", "叶尔",
    "哈兰", "叶尔江", "叶尔肯", "阿热", "阿依", "古丽", "米吉提", "吐尔",
    "巴图", "巴雅", "巴根", "乌兰", "哈斯", "苏林", "其其格",
    "扎西", "多吉", "丹增", "次仁", "格桑", "晋美", "阿旺", "索朗"
]

MINORITY_LAST_NAMES = [
    "艾力", "阿巴斯", "买买提", "阿里木", "卡哈尔", "吾甫尔", "玉苏甫",
    "江", "别克", "阿吉", "木合塔尔", "叶尔江", "肯杰", "阿布都",
    "巴图", "巴雅尔", "巴根", "乌兰", "哈斯", "苏林", "其其格",
    "扎西", "多吉", "丹增", "次仁", "格桑", "晋美", "洛桑"
]

# Foreign player names
FOREIGN_FIRST = {
    "Brazil": ["Lucas", "Gabriel", "Pedro", "Felipe", "Matheus", "Bruno", "Rafael", "Vinicius", "Gustavo", "Marcos", "Andre", "Diego"],
    "Argentina": ["Santiago", "Gonzalo", "Facundo", "Mateo", "Agustin", "Lautaro", "Nicolas", "Martin", "Julian", "Diego", "Ezequiel", "Walter"],
    "Europe": ["James", "Michael", "David", "Thomas", "Alexander", "Daniel", "Marcus", "Kevin", "Ryan", "Chris", "Ivan", "Pavel"]
}

FOREIGN_LAST = {
    "Brazil": ["Silva", "Santos", "Oliveira", "Souza", "Costa", "Pereira", "Lima", "Alves", "Carvalho", "Rodrigues", "Nunes", "Barbosa"],
    "Argentina": ["Martinez", "Lopez", "Gonzalez", "Rodriguez", "Perez", "Sanchez", "Ramirez", "Torres", "Flores", "Rivera", "Medina", "Vargas"],
    "Europe": ["Smith", "Johnson", "Williams", "Brown", "Jones", "Wilson", "Taylor", "Anderson", "Thomas", "Jackson", "Muller", "Weber"]
}

def generate_name(is_foreign=False, is_minority=False, nationality=None):
    if is_foreign:
        nat = nationality or random.choice(["Brazil", "Argentina", "Europe"])
        first = random.choice(FOREIGN_FIRST.get(nat, FOREIGN_FIRST["Europe"]))
        last = random.choice(FOREIGN_LAST.get(nat, FOREIGN_LAST["Europe"]))
        return f"{first} {last}", f"{last}", nat, nat
    elif is_minority:
        first = random.choice(MINORITY_FIRST_NAMES)
        last = random.choice(MINORITY_LAST_NAMES)
        return f"{first}{last}", f"{last}", "China", "China"
    else:
        # Chinese Han players - mix of different name lengths
        name_type = random.random()
        if name_type < 0.35:
            # Two-character name: 姓 + 单字名 (e.g., 王伟, 张磊)
            last = random.choice(CHINESE_LAST_1CHAR)
            first = random.choice(CHINESE_FIRST_2CHAR)
            full_name = f"{last}{first}"
        elif name_type < 0.70:
            # Two-character name: 姓 + 双字名 (e.g., 王浩然, 张子轩)
            last = random.choice(CHINESE_LAST_1CHAR)
            first = random.choice(CHINESE_FIRST_3CHAR)
            full_name = f"{last}{first}"
        else:
            # Three-character name: 双字姓 + 名 (e.g., 欧阳浩然, 司马子轩)
            last = random.choice(CHINESE_LAST_2CHAR)
            first = random.choice(CHINESE_FIRST_2CHAR)
            full_name = f"{last}{first}"
        match_name = f"{last}{first}"
        return full_name, match_name, "China", "China"

def generate_attributes(position, base_ovr, variance=6):
    """Generate player attributes based on position and overall rating."""
    attrs = {}
    
    for attr in ["pace", "stamina", "strength", "agility", "passing", "shooting", 
                 "tackling", "dribbling", "defending", "positioning", "vision", "decisions",
                 "composure", "aggression", "teamwork", "leadership", "handling", "reflexes", "aerial"]:
        attrs[attr] = max(35, min(99, base_ovr + random.randint(-variance, variance)))
    
    position_boosts = {
        "Goalkeeper": ["reflexes", "handling", "aerial", "positioning", "decisions"],
        "CenterBack": ["defending", "strength", "aerial", "tackling", "positioning"],
        "LeftBack": ["pace", "defending", "stamina", "passing", "positioning"],
        "RightBack": ["pace", "defending", "stamina", "passing", "positioning"],
        "DefensiveMidfielder": ["passing", "tackling", "stamina", "defending", "positioning"],
        "CentralMidfielder": ["passing", "stamina", "vision", "decisions", "composure"],
        "AttackingMidfielder": ["passing", "shooting", "dribbling", "vision", "composure"],
        "LeftWinger": ["pace", "dribbling", "shooting", "crossing", "composure"],
        "RightWinger": ["pace", "dribbling", "shooting", "crossing", "composure"],
        "Striker": ["shooting", "pace", "dribbling", "composure", "positioning"]
    }
    
    boosts = position_boosts.get(position, position_boosts["CentralMidfielder"])
    for attr in boosts[:3]:
        attrs[attr] = min(99, attrs[attr] + random.randint(5, 12))
    
    return attrs

def generate_contract_end():
    """Generate contract end date spread throughout the year."""
    year = random.choices([2027, 2028, 2029, 2030], weights=[3, 4, 3, 1])[0]
    month_weights = [2, 1, 1, 2, 3, 4, 5, 4, 3, 2, 1, 3]
    month = random.choices(range(1, 13), weights=month_weights)[0]
    day = random.randint(1, 28)
    return f"{year}-{month:02d}-{day:02d}"

def generate_player(team_id, position, base_ovr, is_foreign=False, is_minority=False, nationality=None):
    """Generate a single player with all required fields."""
    full_name, match_name, nationality, football_nation = generate_name(is_foreign, is_minority, nationality)
    
    # Generate birth date (18-35 years old)
    age = random.randint(18, 35)
    birth_year = 2026 - age
    birth_month = random.randint(1, 12)
    birth_day = random.randint(1, 28)
    dob = f"{birth_year}-{birth_month:02d}-{birth_day:02d}"
    
    # Generate attributes
    attrs = generate_attributes(position, base_ovr)
    
    # Market value based on OVR and age
    age_factor = max(0.6, 1.4 - (age - 18) * 0.04)
    market_value = int((sum(attrs.values()) // len(attrs) ** 2.2) * 200 * age_factor)
    
    # Wage based on market value
    wage = max(1000, int(market_value / 300))
    if is_foreign:
        wage = int(wage * 2.2)
    
    # Contract
    contract_end = generate_contract_end()
    
    return {
        "id": f"p_{team_id}_{position.lower()}_{random.randint(1000, 9999)}",
        "match_name": match_name,
        "full_name": full_name,
        "date_of_birth": dob,
        "nationality": nationality,
        "football_nation": football_nation,
        "birth_country": football_nation,
        "position": position,
        "natural_position": position,
        "alternate_positions": [],
        "footedness": random.choice(["Left", "Right"]),
        "weak_foot": random.randint(2, 5),
        "attributes": attrs,
        "condition": random.randint(70, 100),
        "morale": random.randint(60, 85),
        "fitness": 75,
        "injury": None,
        "team_id": team_id,
        "traits": [],
        "contract_end": contract_end,
        "wage": wage,
        "market_value": market_value,
        "stats": {
            "appearances": 0,
            "goals": 0,
            "assists": 0,
            "clean_sheets": 0,
            "yellow_cards": 0,
            "red_cards": 0,
            "avg_rating": 0.0,
            "minutes_played": 0,
            "shots": 0,
            "shots_on_target": 0,
            "passes_completed": 0,
            "passes_attempted": 0,
            "tackles_won": 0,
            "interceptions": 0,
            "fouls_committed": 0
        },
        "career": [],
        "training_focus": None,
        "transfer_listed": False,
        "loan_listed": False,
        "transfer_offers": [],
        "morale_core": {
            "manager_trust": 50,
            "unresolved_issue": None,
            "recent_treatment": None,
            "pending_promise": None,
            "talk_cooldown_until": None,
            "renewal_state": None
        }
    }

def generate_team_squad(team, has_foreigners=True):
    """Generate a balanced squad for a team."""
    players = []
    staff = []
    
    team_id = team["id"]
    is_zhejiang = "浙江" in team["name"] or "Zhejiang" in team["name"]
    
    if is_zhejiang or not has_foreigners:
        max_foreigners = 0
    else:
        max_foreigners = random.randint(2, 5)
    
    max_minorities = random.randint(1, 3)
    foreign_count = 0
    minority_count = 0
    
    base_ovr = 58 + random.randint(-8, 8)
    
    positions = [
        "Goalkeeper",
        "CenterBack", "CenterBack",
        "LeftBack", "RightBack",
        "CentralMidfielder", "CentralMidfielder", "CentralMidfielder", "CentralMidfielder",
        "Striker", "Striker"
    ]
    
    for i, pos in enumerate(positions):
        is_foreign = False
        is_minority = False
        
        if foreign_count < max_foreigners and random.random() < 0.35:
            is_foreign = True
            foreign_count += 1
            nat = random.choice(["Brazil", "Argentina", "Europe"])
            player = generate_player(team_id, pos, base_ovr + random.randint(10, 18), 
                                   is_foreign=True, nationality=nat)
        elif minority_count < max_minorities and random.random() < 0.25:
            is_minority = True
            minority_count += 1
            player = generate_player(team_id, pos, base_ovr + random.randint(-2, 5), 
                                   is_minority=True)
        else:
            player = generate_player(team_id, pos, base_ovr + random.randint(-4, 6))
        
        player["starting_xi"] = True
        players.append(player)
    
    bench_pool = ["Goalkeeper", "CenterBack", "LeftBack", "RightBack", 
                  "CentralMidfielder", "AttackingMidfielder", "LeftWinger", "RightWinger", "Striker"]
    
    for i in range(14):
        pos = random.choice(bench_pool)
        is_foreign = False
        is_minority = False
        
        if foreign_count < max_foreigners and random.random() < 0.3:
            is_foreign = True
            foreign_count += 1
            nat = random.choice(["Brazil", "Argentina", "Europe"])
            player = generate_player(team_id, pos, base_ovr + random.randint(5, 15), 
                                    is_foreign=True, nationality=nat)
        elif minority_count < max_minorities and random.random() < 0.2:
            is_minority = True
            minority_count += 1
            player = generate_player(team_id, pos, base_ovr + random.randint(-5, 3), 
                                    is_minority=True)
        else:
            player = generate_player(team_id, pos, base_ovr + random.randint(-8, 2))
        
        players.append(player)
    
    coach_first = random.choice(["John", "Mike", "David", "Carlos", "Paulo", "张", "王", "李", "刘", "陈"])
    coach_last = random.choice(["Smith", "Brown", "Wilson", "Silva", "Santos", "伟", "强", "磊", "浩", "杰"])
    staff.append({
        "id": f"s_{team_id}_coach",
        "first_name": coach_first,
        "last_name": coach_last,
        "date_of_birth": f"{random.randint(1960, 1980)}-{random.randint(1, 12):02d}-{random.randint(1, 28):02d}",
        "nationality": "China",
        "football_nation": "China",
        "birth_country": "China",
        "role": "Coach",
        "attributes": {
            "coaching": random.randint(50, 90),
            "judging_ability": random.randint(50, 80),
            "judging_potential": random.randint(40, 70),
            "physiotherapy": random.randint(20, 60)
        },
        "team_id": team_id,
        "specialization": None,
        "wage": random.randint(50000, 200000),
        "contract_end": generate_contract_end()
    })
    
    if random.random() < 0.8:
        staff.append({
            "id": f"s_{team_id}_assistant",
            "first_name": "Assistant",
            "last_name": "Coach",
            "date_of_birth": f"{random.randint(1970, 1985)}-{random.randint(1, 12):02d}-{random.randint(1, 28):02d}",
            "nationality": "China",
            "football_nation": "China",
            "birth_country": "China",
            "role": "AssistantManager",
            "attributes": {
                "coaching": random.randint(40, 70),
                "judging_ability": random.randint(40, 70),
                "judging_potential": random.randint(40, 70),
                "physiotherapy": random.randint(20, 60)
            },
            "team_id": team_id,
            "specialization": None,
            "wage": random.randint(20000, 80000),
            "contract_end": generate_contract_end()
        })
    
    if random.random() < 0.9:
        staff.append({
            "id": f"s_{team_id}_physio",
            "first_name": "Physio",
            "last_name": f"{random.randint(1, 100)}",
            "date_of_birth": f"{random.randint(1980, 1995)}-{random.randint(1, 12):02d}-{random.randint(1, 28):02d}",
            "nationality": "China",
            "football_nation": "China",
            "birth_country": "China",
            "role": "Physio",
            "attributes": {
                "coaching": random.randint(10, 40),
                "judging_ability": random.randint(20, 50),
                "judging_potential": random.randint(20, 50),
                "physiotherapy": random.randint(50, 90)
            },
            "team_id": team_id,
            "specialization": None,
            "wage": random.randint(10000, 40000),
            "contract_end": generate_contract_end()
        })
    
    if random.random() < 0.6:
        staff.append({
            "id": f"s_{team_id}_scout",
            "first_name": "Scout",
            "last_name": f"{random.randint(1, 100)}",
            "date_of_birth": f"{random.randint(1975, 1990)}-{random.randint(1, 12):02d}-{random.randint(1, 28):02d}",
            "nationality": "China",
            "football_nation": "China",
            "birth_country": "China",
            "role": "Scout",
            "attributes": {
                "coaching": random.randint(20, 50),
                "judging_ability": random.randint(60, 90),
                "judging_potential": random.randint(60, 90),
                "physiotherapy": random.randint(10, 40)
            },
            "team_id": team_id,
            "specialization": None,
            "wage": random.randint(15000, 50000),
            "contract_end": generate_contract_end()
        })
    
    return players, staff, foreign_count, minority_count

def main():
    with open("chinese_super_league.json", "r", encoding="utf-8") as f:
        data = json.load(f)
    
    all_players = []
    all_staff = []
    
    for team in data["teams"]:
        print(f"Generating squad for {team['name']}...")
        
        players, staff, foreigners, minorities = generate_team_squad(team)
        all_players.extend(players)
        all_staff.extend(staff)
        
        print(f"  - Players: {len(players)}, Foreigners: {foreigners}, Minorities: {minorities}")
    
    data["players"] = all_players
    data["staff"] = all_staff
    
    with open("chinese_super_league.json", "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    
    print(f"\nTotal: {len(all_players)} players, {len(all_staff)} staff")

if __name__ == "__main__":
    main()
