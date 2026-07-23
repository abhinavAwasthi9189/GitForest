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

    for (month, repo_counts) in &month_based_repo_count {
        println!("{month}:");
        for (repo, count) in repo_counts {
            println!("  {repo}: {count} commits");
        }
    }

    println!("so now we see the one after finishing");

    let month_based_repo_count = future_proof(&mut month_based_repo_count);
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
    let repos = octocrab.users(username.clone()).repos().send().await?;
    for getrepo in repos {
        let repo = octocrab
            .repos(username.clone(), getrepo.name.clone())
            .list_commits()
            .author(username.clone())
            .send()
            .await?;
        //here we take the repo from github and then go through the commits
        for commitset in repo {
            //if we have an author and date for the commit. we print it.
            if let Some(commit) = &commitset.commit.author {
                if let Some(date) = &commit.date {
                    let date = date.format("%Y-%m").to_string();
                    month_based_repo_count.entry(date).or_insert_with(BTreeMap::new).entry(getrepo.name.clone()).and_modify(|val| *val += 1).or_insert(1);
                }
            }
        }
    }
    Ok(month_based_repo_count)
}

fn future_proof(tree: &mut BTreeMap<String, BTreeMap<String, i32>>) -> BTreeMap<String,BTreeMap<String,i32>>{
    let mut treecopy = tree.clone();
    let key_set:Vec<String> = treecopy.keys().cloned().collect();
    for (month, repo_counts) in tree.into_iter() {
        for (repo, count) in repo_counts {
             
            for monthcopy in &key_set{
                if month<monthcopy{
                    treecopy.entry(monthcopy.clone()).or_insert_with(BTreeMap::new).entry(repo.clone()).and_modify(|val| *val+=*count).or_insert(count.clone());                }
            }
        }
    }

    treecopy
}

fn strput() -> String {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}
