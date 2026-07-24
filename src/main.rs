use octocrab;
use std::collections::BTreeMap;
use std::env;
use tokio;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("give us the username");
    let username = strput();
    let gt = tokio::runtime::Runtime::new()?;
    let mut month_based_repo_count = gt.block_on(async { get_git(&username).await })?;
    println!("done!!!");

    println!("so now we see the one after finishing");

    let first = month_based_repo_count.keys().next().clone().unwrap();
    let last = month_based_repo_count.keys().next_back().clone().unwrap();
    let month_range = month_range(first,last);

    for i in &month_range{
        println!("{}",i);
    }
    
    //right now all commits are the month that they happened not any other one. this helps us do that.
    let month_based_repo_count = future_proof(&mut month_based_repo_count,month_range);
    for (month, repo_counts) in &month_based_repo_count {
        println!("{month}:");
        for (repo, count) in repo_counts {
            println!("  {repo}: {count} commits");
        }
    }
    

    Ok(())
}

//we have made a seprate async function so that everything async can be put in a seprate bucket
async fn get_git(
    username: &String,
) -> Result<BTreeMap<String, BTreeMap<String, i32>>, Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(env::var("GITHUB_TOKEN")?)
        .build()?;

    let mut month_based_repo_count: BTreeMap<String, BTreeMap<String, i32>> = BTreeMap::new();

    //here we get a list of all the repos that the user has
    let repo_pages = octocrab.users(username.clone()).repos().per_page(100).send().await?;
    let repo = octocrab.all_pages(repo_pages).await?;
    for getrepo in repo {
        let commit_page = octocrab
            .repos(username.clone(), getrepo.name.clone())
            .list_commits()
            .author(username.clone())
            .per_page(100)
            .send()
            .await?;
        //here we take the repo from github and then go through the commits
        let allcommits = octocrab.all_pages(commit_page).await?;
        for commitset in allcommits {
            //if we have an author and date for the commit. we print it.
            if let Some(commit) = &commitset.commit.author {
                if let Some(date) = &commit.date {
                    //only take the date and month from date of commit and adds it to the BTreeMap
                    let date = date.format("%Y-%m").to_string();
                    month_based_repo_count.entry(date).or_insert_with(BTreeMap::new).entry(getrepo.name.clone()).and_modify(|val| *val += 1).or_insert(1);
                }
            }
        }
    }
    Ok(month_based_repo_count)
}

fn future_proof(tree: &BTreeMap<String, BTreeMap<String, i32>>, key_set: Vec<String>) -> BTreeMap<String, BTreeMap<String, i32>> {
    let mut treecopy: BTreeMap<String, BTreeMap<String, i32>> = BTreeMap::new();

    for label in &key_set {
        treecopy.entry(label.clone()).or_insert_with(BTreeMap::new);
    }

    for (month, repo_counts) in tree.iter() {
        for key in &key_set{
            if key>=month{
                for (repo, count) in repo_counts {
                    treecopy
                    .entry(key.clone())
                    .or_insert_with(BTreeMap::new)
                    .entry(repo.clone())
                    .and_modify(|val| *val += *count)
                    .or_insert(*count);
                }
            }
        }
    }

    treecopy
}

fn month_range(start: &str, end: &str) -> Vec<String> {
    let parse = |s: &str| -> (i32, u32) {
        let parts: Vec<&str> = s.split('-').collect();
        (parts[0].parse().unwrap(), parts[1].parse().unwrap())
    };

    let (end_y, end_m) = parse(start);
    let (mut y, mut m) = parse(end);
    let sizeset =[1,3,6]; 
    let mut months = Vec::new();
    let mut size = sizeset[0];
    let mut i =0;
    'outer:loop {
        if i == 10 && size == sizeset[0] {
            size = sizeset[1];
        }
        else if i == 20 && size == sizeset[1] {
            size = sizeset[2];
        }
        months.push(format!("{:04}-{:02}", y, m));
        for _ in 0..size{
            if y < end_y || (y == end_y && m <= end_m){
                break 'outer;
            }
            if m==1{y-=1;m=12;continue;}
            m-=1;
        }
        i+=1;
    }
    months
}

fn strput() -> String {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}
